use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum SecretLoadError {
    EnvVarNotSet {
        name: String,
    },
    ArnFetchFailed {
        name: String,
        arn: String,
        aws_error: String,
    },
}

impl fmt::Display for SecretLoadError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SecretLoadError::EnvVarNotSet { name } => {
                write!(
                    f,
                    "Environment variable '{}' is not set.\n\
                     \n\
                     Troubleshooting:\n\
                     1. Check that the environment variable is defined in your Lambda configuration\n\
                     2. For local development, ensure it's in your .env file\n\
                     3. Verify the variable name is spelled correctly",
                    name
                )
            }
            SecretLoadError::ArnFetchFailed {
                name,
                arn,
                aws_error,
            } => {
                write!(
                    f,
                    "Failed to load secret from AWS Secrets Manager\n\
                     \n\
                     Environment Variable: {}\n\
                     ARN: {}\n\
                     AWS Error: {}\n\
                     \n\
                     This error means the environment variable contains an ARN but the actual secret\n\
                     could not be fetched from AWS Secrets Manager.\n\
                     \n\
                     Troubleshooting:\n\
                     1. Verify Lambda execution role has 'secretsmanager:GetSecretValue' permission\n\
                     2. Check that the secret exists in AWS Secrets Manager\n\
                     3. Verify the ARN is correct and not a partial/malformed ARN\n\
                     4. Check CloudWatch logs for additional AWS error details\n\
                     5. Verify the secret is in the same region as the Lambda function",
                    name, arn, aws_error
                )
            }
        }
    }
}

impl Error for SecretLoadError {}

fn is_secrets_manager_arn(value: &str) -> bool {
    value.starts_with("arn:aws:secretsmanager:")
}

pub async fn create_secrets_client() -> aws_sdk_secretsmanager::Client {
    let config =
        aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    aws_sdk_secretsmanager::Client::new(&config)
}

pub async fn get_secret(name: &str) -> Result<String, Box<dyn Error>> {
    if name == "LOCAL_REDIS_URL" {
        return Ok("redis://localhost:6379".to_string());
    }

    let env_value = match std::env::var(name) {
        Ok(val) => val,
        Err(_) => {
            return Err(Box::new(SecretLoadError::EnvVarNotSet {
                name: name.to_string(),
            }));
        }
    };

    if is_secrets_manager_arn(&env_value) {
        tracing::info!(
            "Environment variable '{}' contains Secrets Manager ARN, fetching actual secret",
            name
        );

        let secrets_client = create_secrets_client().await;

        match load_secret(&secrets_client, &env_value).await {
            Ok(actual_value) => {
                tracing::info!("Successfully loaded secret for '{}'", name);
                Ok(actual_value)
            }
            Err(e) => {
                tracing::error!(
                    "Failed to fetch secret from Secrets Manager for '{}': {}",
                    name,
                    e
                );

                Err(Box::new(SecretLoadError::ArnFetchFailed {
                    name: name.to_string(),
                    arn: env_value,
                    aws_error: e.to_string(),
                }))
            }
        }
    } else {
        tracing::debug!(
            "Using direct value for environment variable '{}'",
            name
        );
        Ok(env_value)
    }
}

async fn load_secret(
    secrets_client: &aws_sdk_secretsmanager::Client,
    secret_id: &str,
) -> Result<String, Box<dyn Error>> {
    let secret_response = secrets_client
        .get_secret_value()
        .secret_id(secret_id)
        .send()
        .await?;

    let secret_string = secret_response
        .secret_string()
        .expect("Secret must have string value");

    Ok(secret_string.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_secrets_manager_arn() {
        // Valid ARNs
        assert!(is_secrets_manager_arn(
            "arn:aws:secretsmanager:us-east-1:123456789:secret:my-secret"
        ));
        assert!(is_secrets_manager_arn(
            "arn:aws:secretsmanager:eu-west-1:999999999:secret:another-secret-abc123"
        ));
        assert!(is_secrets_manager_arn(
            "arn:aws:secretsmanager:us-west-2:111111111:secret:test-123"
        ));

        // Not ARNs
        assert!(!is_secrets_manager_arn("actual-secret-value"));
        assert!(!is_secrets_manager_arn("redis://localhost:6379"));
        assert!(!is_secrets_manager_arn(""));
        assert!(!is_secrets_manager_arn("arn:aws:s3:::my-bucket"));
        assert!(!is_secrets_manager_arn("arn:aws:iam::123456789:user/test"));
    }

    #[test]
    fn test_error_message_formatting_env_var_not_set() {
        let err = SecretLoadError::EnvVarNotSet {
            name: "TEST_VAR".to_string(),
        };
        let msg = format!("{}", err);

        // Verify key components are in the error message
        assert!(msg.contains("TEST_VAR"));
        assert!(msg.contains("not set"));
        assert!(msg.contains("Troubleshooting"));
        assert!(msg.contains("Lambda configuration"));
        assert!(msg.contains(".env file"));
    }

    #[test]
    fn test_error_message_formatting_arn_fetch_failed() {
        let err = SecretLoadError::ArnFetchFailed {
            name: "JUPITER_API_KEY".to_string(),
            arn: "arn:aws:secretsmanager:us-east-1:123:secret:test".to_string(),
            aws_error: "AccessDeniedException: User is not authorized"
                .to_string(),
        };
        let msg = format!("{}", err);

        // Verify key components are in the error message
        assert!(msg.contains("JUPITER_API_KEY"));
        assert!(msg.contains("arn:aws:secretsmanager"));
        assert!(msg.contains("AccessDeniedException"));
        assert!(msg.contains("Troubleshooting"));
        assert!(msg.contains("secretsmanager:GetSecretValue"));
        assert!(msg.contains("Lambda execution role"));
    }

    #[test]
    fn test_error_message_includes_all_troubleshooting_steps() {
        let err = SecretLoadError::ArnFetchFailed {
            name: "TEST_SECRET".to_string(),
            arn: "arn:aws:secretsmanager:us-east-1:123:secret:test".to_string(),
            aws_error: "Test error".to_string(),
        };
        let msg = format!("{}", err);

        // Verify all 5 troubleshooting steps are present
        assert!(msg.contains("1. Verify Lambda execution role"));
        assert!(msg.contains("2. Check that the secret exists"));
        assert!(msg.contains("3. Verify the ARN is correct"));
        assert!(msg.contains("4. Check CloudWatch logs"));
        assert!(msg.contains("5. Verify the secret is in the same region"));
    }

    #[test]
    fn test_print_example_error_messages() {
        println!("\n========================================");
        println!("EXAMPLE ERROR MESSAGE: Environment Variable Not Set");
        println!("========================================\n");

        let err1 = SecretLoadError::EnvVarNotSet {
            name: "JUPITER_API_KEY".to_string(),
        };
        println!("{}\n", err1);

        println!("========================================");
        println!("EXAMPLE ERROR MESSAGE: ARN Fetch Failed");
        println!("========================================\n");

        let err2 = SecretLoadError::ArnFetchFailed {
            name: "JUPITER_API_KEY".to_string(),
            arn: "arn:aws:secretsmanager:us-east-1:123456789012:secret:jupiter-api-key-abc123".to_string(),
            aws_error: "AccessDeniedException: User: arn:aws:sts::123456789012:assumed-role/lambda-execution-role/rebalance-paymasters is not authorized to perform: secretsmanager:GetSecretValue on resource: arn:aws:secretsmanager:us-east-1:123456789012:secret:jupiter-api-key-abc123".to_string(),
        };
        println!("{}\n", err2);

        println!("========================================");
        println!("EXAMPLE ERROR MESSAGE: Invalid ARN");
        println!("========================================\n");

        let err3 = SecretLoadError::ArnFetchFailed {
            name: "HELIUS_MAINNET_API_KEY".to_string(),
            arn: "arn:aws:secretsmanager:us-west-2:999999999999:secret:helius-key".to_string(),
            aws_error: "ResourceNotFoundException: Secrets Manager can't find the specified secret".to_string(),
        };
        println!("{}\n", err3);
    }
}

# Tiltfile - Simple configuration
docker_compose("./docker-compose.yml")

# API services
dc_resource('api-server', trigger_mode = TRIGGER_MODE_MANUAL, labels=['api'])

# Database and cache
dc_resource('postgresql-db', labels=['database'])
dc_resource('pgadmin', labels=['database'])
dc_resource('redis', labels=['cache'])
dc_resource('redisinsight', labels=['cache'])
dc_resource('redis-ui', labels=['cache'])

# Monitoring
dc_resource('prometheus', labels=['monitoring'])
dc_resource('grafana', labels=['monitoring'])
dc_resource('postgres-exporter', labels=['monitoring'])
dc_resource('redis-exporter', labels=['monitoring'])
dc_resource('prometheus-pushgateway', labels=['monitoring'])
dc_resource('node-exporter', labels=['monitoring'])
dc_resource('cadvisor', labels=['monitoring'])
dc_resource('alertmanager', labels=['monitoring'])
dc_resource('grafana-image-renderer', labels=['monitoring'])

# Initialize last_indexed_slot table with current slot for DEVNET
local_resource(
    'db-migrations',
    cmd='./scripts/run_migration.sh',
    resource_deps=['postgresql-db'],
    trigger_mode=TRIGGER_MODE_MANUAL,
    labels=['database'],
    auto_init=False
)

# ignore container restarts when these directories change
watch_settings(
    ignore=[
    '/data/**',
    '/data/postgres/*',
    '/data/postgres/**',
    '/data/postgres/pg_wal/**', # Explicitly ignore WAL files
    '**/pg_wal/**',
    '.git/**',
    'target/**',
    'node_modules/**',
    'tests/**',
    ],
)

# Enable BuildKit for faster builds
os.environ['COMPOSE_DOCKER_CLI_BUILD'] = '1'
os.environ['DOCKER_BUILDKIT'] = '1'

# Define configuration options
config.define_string_list("to-run", args=True)
cfg = config.parse()

# Define service groups
groups_by_team = {
    'api': ['api-server', 'postgresql-db', 'pgadmin', 'redis', 'redisinsight', 'redis-ui', 'postgres-exporter', 'redis-exporter', 'db-migrations', 'prometheus', 'prometheus-pushgateway', 'grafana', 'grafana-image-renderer', 'node-exporter', 'cadvisor', 'alertmanager'],
    'local-api-testing': ['postgresql-db', 'pgadmin', 'redis', 'redisinsight', 'redis-ui', 'postgres-exporter', 'redis-exporter', 'db-migrations', 'prometheus', 'prometheus-pushgateway', 'grafana', 'grafana-image-renderer', 'node-exporter', 'cadvisor', 'alertmanager'],
}

# Process resources
all_resources = set()
for team, resources in groups_by_team.items():
    for resource in resources:
        all_resources.add(resource)

# Determine which resources to run and environment
to_run = []
for arg in cfg.get('to-run', []):
    if arg in groups_by_team:
        to_run.extend(groups_by_team.get(arg))
    else:
        to_run.append(arg)

# Enable selected resources
config.set_enabled_resources(to_run)

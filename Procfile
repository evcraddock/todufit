# todu-fit development services
# Usage: ENV_FILE=$PWD/.env overmind start -s .overmind.sock -D

vite: cd web && set -a && source ${ENV_FILE} && set +a && npm run dev
hono: cd web && set -a && source ${ENV_FILE} && set +a && npm run dev:server
sync: set -a && source ${ENV_FILE} && set +a && docker compose up sync-server

## Database setup docs

Install docker and docker compose.
Install https://direnv.net/.

```
cp .env.development.sample .env.development.database
cp docker-compose.yml.sample docker-compose.yml
cp envrc.sample .envrc
direnv allow
docker-compose up
diesel setup
```

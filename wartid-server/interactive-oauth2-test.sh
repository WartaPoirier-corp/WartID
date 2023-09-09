#!/bin/bash

token_url=http://localhost:8000/oauth2/token
userinfo_url=http://localhost:8000/oauth2/userinfo

# Load dotenv
if [ -f .env ]; then
  export $(cat .env | grep -v '#' | awk '/=/ {print $1}')
fi

# Check if DATABASE_URL exists
if [ -z "$DATABASE_URL" ]; then
  echo "No DATABASE_URL set or found in \`.env\`. Aborting"
  exit 1
fi

# Obtain "constants"
client_id_def=$(psql -qtAX "$DATABASE_URL" -c "SELECT (id) FROM user_apps LIMIT 1")
read -r -p "Client ID (default: $client_id_def): " client_id
client_id=${client_id:-$client_id_def}
client_secret=$(psql -qtAX "$DATABASE_URL" -c "SELECT (oauth_secret) FROM user_apps WHERE id = '$client_id' LIMIT 1")
python -c "print('Client Secret (found in database): ' + ('*' * ${#client_secret}))"
client_redirect=$(psql -qtAX "$DATABASE_URL" -c "SELECT (oauth_redirect) FROM user_apps WHERE id = '$client_id' LIMIT 1")
echo "Client Redirect (found in database): $client_redirect"

echo -----

while true
do

  echo "What do you want to do ?"
  echo "  - Obtain tokens from an (a)uthorization code"
  echo "  - Obtain tokens from a (r)efresh token"
  echo "  - (T)est you access token"
  echo "  - (Q)uit"

  read -r -n1 -p "> " choice
  echo

  if [ "$choice" = "a" ] || [ "$choice" = "A" ]; then
    read -r -p "Authorization code = " code
    curl -s \
      --data-urlencode "grant_type=authorization_code" \
      --data-urlencode "code=$code" \
      --data-urlencode "client_id=$client_id" \
      --data-urlencode "client_secret=$client_secret" \
      --data-urlencode "redirect_uri=$client_redirect" \
      $token_url | jq
  elif [ "$choice" = "r" ] || [ "$choice" = "R" ]; then
    read -r -p "Refresh token = " refresh_token
    curl -s \
      --data-urlencode "grant_type=refresh_token" \
      --data-urlencode "refresh_token=$refresh_token" \
      --data-urlencode "client_id=$client_id" \
      --data-urlencode "client_secret=$client_secret" \
      --data-urlencode "redirect_uri=$client_redirect" \
      $token_url | jq
  elif [ "$choice" = "t" ] || [ "$choice" = "T" ]; then
    read -r -p "Access token = " access_token
    curl -s \
      -H "Authorization: Bearer $access_token" \
      $userinfo_url \
      | jq
  elif [ "$choice" = "q" ] || [ "$choice" = "Q" ]; then
    exit
  fi

  echo -----

done

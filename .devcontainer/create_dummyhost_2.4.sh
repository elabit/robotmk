#!/bin/bash

SECRETFILE=/opt/omd/sites/cmk/var/check_mk/web/automation/automation.secret

if [ ! -r $SECRETFILE ]; then 
    echo "ERROR: In order to create a dummy host with this script, you must first create an automation user and store the secret in clear text!"
    exit 1
fi

CMK_HOST="localhost"
SITE_NAME="cmk"
HOST="$HOSTNAME"
PROTO="http"
PORT=5000
API_URL="$PROTO://$CMK_HOST:$PORT/$SITE_NAME/check_mk/api/1.0"

USERNAME="automation"
PASSWORD=$(cat $SECRETFILE)

echo "Automation password: $PASSWORD"
echo "+ Creating a dummy host via API... "

curl \
  --request POST \
  --header "Authorization: Bearer $USERNAME $PASSWORD" \
  --header "Accept: application/json" \
  --header "Content-Type: application/json" \
  --data '{"attributes": {"ipaddress": "127.0.0.1"},"folder": "/","host_name": "'$HOST'"}' \
  "$API_URL/domain-types/host_config/collections/all"
cmk -R


if ! $(grep -q robotmk /omd/sites/cmk/etc/check_mk/conf.d/wato/rules.mk); then
    echo "+ Adding rules.mk, replacing HOSTNAME with $HOST via envsubst ... "
    
    CFG=$(envsubst < /workspaces/robotmk/.devcontainer/rules.mk.txt)

    echo "$CFG" >> /omd/sites/cmk/etc/check_mk/conf.d/wato/rules.mk  
else 
    echo
    echo "+ robotmk agent config already in rules.mk ... "
fi


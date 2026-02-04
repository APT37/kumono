#!/usr/bin/fish

# This script determines the valid charset for a given service.
# 
# Make sure to install the following programs: fish, ripgrep, jq, sort
# 
# Usage:
# ./charset.fish coomer.st onlyfans
# ./charset.fish kemono.cr patreon

for char in (curl -sS "https://$argv[1]/api/v1/creators.txt" | jq -c .[] | rg $argv[2] | jq -r .id | string split '' | sort -u)
    echo -n $char
end

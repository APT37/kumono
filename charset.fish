#!/usr/bin/fish

# ./charset.fish coomer onlyfans

for char in (curl -sS https://$argv[1].su/api/v1/creators.txt | jq -c .[] | rg $argv[2] | jq -r .id | string split '' | sort -u)
    echo -n $char
end

#!/usr/bin/fish

# ./charset.fish coomer.st onlyfans
# ./charset.fish kemono.cr patreon

for char in (curl -sS https://$argv[1]/api/v1/creators.txt | jq -c .[] | rg $argv[2] | jq -r .id | string split '' | sort -u)
    echo -n $char
end

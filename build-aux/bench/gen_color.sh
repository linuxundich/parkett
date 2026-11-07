#!/bin/sh
T=$1
date +%s.%N > $T/start
awk 'BEGIN{for(i=0;i<800000;i++) printf "\033[31mred\033[0m \033[1;32mgreen\033[0m \033[4munder\033[0m plain text of line %d with 漢字\n", i}'
date +%s.%N > $T/end
sleep 20

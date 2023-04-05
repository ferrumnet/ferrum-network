#!/bin/bash
TIME=$(date '+%F-%H%M%S')
echo "###################################"
echo "#### Script execute at ${TIME} ####"
echo "###################################"
docker system prune -f
docker stop  $(docker ps --filter "name=ferrum-network*" -q)
aws ecr get-login-password --region us-east-2 | docker login --username AWS --password-stdin 806611346442.dkr.ecr.us-east-2.amazonaws.com
docker rmi  -f $(docker images --filter=reference=806611346442.dkr.ecr.us-east-2.amazonaws.com/ferrum_node:* -q)
aws ecr get-login-password --region us-east-2 | docker login --username AWS --password-stdin 806611346442.dkr.ecr.us-east-2.amazonaws.com
docker  run -d -it  -p 30333:30333 -p  9933:9933  -p 9944:9944  -p  9615:9615 --name ferrum-network-${TIME} 806611346442.dkr.ecr.us-east-2.amazonaws.com/ferrum_node:latest
echo "###################################"
echo "#### Ended ####"
echo "###################################"
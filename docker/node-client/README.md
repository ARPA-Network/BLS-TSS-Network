# Node Client Updates

## Todo
- [ ] Modifying node-client docker image to allow restarting with existing config, log, and db
- [ ] Adding flag for local vs dockerhub deployment
- [ ] Test docker local sqlite: Try stopping and starting nodes with docker and see if deploying new user contract and request randomnesss is still working. 


## What was done.
- Moved node-client docker folder to /docker/
- Moved internet-test, localnet-test, and mainnet code to /docker/deployments/

## Mainnet Node re-deployment 

This is for when:
- You have existing running nodes, that need to be updated to new node client.
- sqlite db and logs need to copied to the new docker containers. 
- config file needs to be changed:
    - Add relayed chains

1. run node containers with config, log, and db as volume mounts: 
`docker run -d --name node1 -p 50061:50061 -p 50091:50091 -v /tmp/BLS-TSS-Network/docker/`
2. edit the config file to reference the correct log and db locations
3. test it out

(try to simplify the volume mount path. Reference the user-shell)
(put arpa node directly under docker)
(tidy up internet-test, localnet-test, mainnet later)


## Manual Build Instructions

```bash
# manually build images
cd BLS-TSS-Network
docker build -t node-client ./docker/internet-test/node-client

# tag imges for docker hub
docker tag node-client arpachainio/node:latest

# push images to docker hub
docker push arpachainio/node-client:latest

# pull images
docker pull arpachainio/node-client:latest
```


## Container Commands

```bash
# Build Locally
cd BLS-TSS-Network
docker build -t node-client ./docker/node-client


# Run Locally
docker run -d \
  --name node1 \
  -p 50061:50061 -p 50091:50091 \
  -v ./docker/localnet-test/node-client/config_1.yml:/app/external/config.yml \
  -v ./docker/localnet-test/node-client/data1.sqlite:/app/external/data.sqlite \
  -v ./docker/localnet-test/node-client/log:/app/external/log \
  node-client:latest

docker run -d \
  --name node2 \
  -p 50061:50061 -p 50091:50091 \
  -v ./docker/localnet-test/node-client/config_2.yml:/app/external/config.yml \
  -v ./docker/localnet-test/node-client/data2.sqlite:/app/external/data.sqlite \
  -v ./docker/localnet-test/node-client/log:/app/external/log \
  node-client:latest

docker run -d \
  --name node3 \
  -p 50061:50061 -p 50091:50091 \
  -v ./docker/localnet-test/node-client/config_3.yml:/app/external/config.yml \
  -v ./docker/localnet-test/node-client/data3.sqlite:/app/external/data.sqlite \
  -v ./docker/localnet-test/node-client/log:/app/external/log \
  node-client:latest

# Kill these docker containers only
docker kill node1 node2 node3
docker rm node1 node2 node3


# Dockerhub
docker run -d --name node1 -p 50061:50061 -p 50091:50091 -v /tmp/BLS-TSS-Network/docker/mainnet/node-client/config_1.yml:/app/external/config.yml arpachainio/node-client:latest

docker run -d --name node2 -p 50062:50061 -p 50092:50091 -v /tmp/BLS-TSS-Network/docker/mainnet/node-client/config_2.yml:/app/external/config.yml arpachainio/node-client:latest

docker run -d --name node3 -p 50063:50061 -p 50093:50091 -v /tmp/BLS-TSS-Network/docker/mainnet/node-client/config_3.yml:/app/external/config.yml arpachainio/node-client:latest
```
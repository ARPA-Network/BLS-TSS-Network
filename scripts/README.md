# ARPA Network L1/L2 Devnet Automation

## Usage

Start Optimism Devnet

```bash
cd optimism
make devnet-up-deploy
```

Deploy ARPA Network contracts to L1 and L2 and start randcast nodes

```bash
cd scripts
# activate venv
python3 -m venv .venv
source .venv/bin/activate
# install dependencies
pip3 install -r requirements.txt
# run script
python3 main.py
```

Kill Existing randcast nodes

```bash
docker kill $(docker ps -q -f ancestxor=arpachainio/node:latest); docker rm -f $(docker ps -a -q -f ancestor=arpachainio/node:latest)
```
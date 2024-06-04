# V0.2.0 Node Client/Operator Update Instructions

## Steps

1. Stop your existing Docker instance and remove it.
2. Database file handling:
- If you are an operator currently experiencing issues, delete your existing database file.
- Otherwise, you can keep your existing database file as long as the running command points to it correctly.

3. Pull the most recent version of the ARPA Node Client Docker image by running the following command:

    ```docker pull ghcr.io/arpa-network/node-client:latest```

4. Start the Docker container by running the following command:

```bash
    docker run -d \
    --name <name of your node> \
    -v <path of your config>:/app/config.yml \
    -v <db folder path if your db file is named as data.sqlite>:/app/db \
    -v ./log:/app/log \
    --network=host \
    ghcr.io/arpa-network/node-client:latest
```
``Note``: If your database file is named differently than "data.sqlite", you need to map it manually using the following format:
    ```-v <db file path>:/app/db/data.sqlite```

5. After about 1 minute, search the log for the keyword "public_key" or "DKGKeyGenerated" and copy the public key value.
6. Generate a signature and call the [nodeRegister contract method](https://github.com/ARPA-Network/BLS-TSS-Network/blob/0732850fe39f869a7dea899e445dfe6332462ab7/contracts/src/interfaces/INodeRegistry.sol#L25) manually with your generated public key. You can find the method definition here.
7. After calling the nodeRegister method, you should expect to see the Node Client log running correctly.
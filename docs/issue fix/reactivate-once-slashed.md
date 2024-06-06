# How To Reactivate Once the Node Slashed/Quitted

## General Guidance

Assuming everything is working properly, all you need to do is to call the contract.

Which means, you are going to need

- [NodeRegistry Contract](https://github.com/ARPA-Network/BLS-TSS-Network/blob/0732850fe39f869a7dea899e445dfe6332462ab7/contracts/src/interfaces/INodeRegistry.sol)
- The address of the contract: listed in our [Official Document](https://docs.arpanetwork.io/randcast/supported-networks-and-parameters), then
- Generate the EIP1271 Operator signature for AVS registration with your `Asset` Account (for details, check our [onboarding doc](/docs/eigenlayer-onboarding.md))
- Call `nodeActivate` method by `Node` account to activate your node again.(if still doesn't work, contact us via telagram group)

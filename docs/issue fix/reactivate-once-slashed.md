# How To Reactivate Once the Node Slashed/Quitted

## General Guidance

Please keep your `node-client` running to avoid missing DKG grouping information after node activation.

For `Node` account that has been quitted or slashed, the corresponding `Asset`(operator) account was also deregistered from `AVSDirectory`, so you need to provide the operator's signature to register it to our AVS again.

To generate the EIP1271 Operator signature for AVS registration with your `Asset` Account, please check our [onboarding doc](/docs/eigenlayer-onboarding.md#generate-the-eip1271-operator-signature-for-avs-registration-with-your-asset-account).

Note: The `Node` account that has been logged out **cannot** be activated again.

Call `nodeActivate` method by `Node` account to activate your node again.(if it still doesn't work, contact us via telagram group):

- [NodeRegistry Contract](https://github.com/ARPA-Network/BLS-TSS-Network/blob/0732850fe39f869a7dea899e445dfe6332462ab7/contracts/src/interfaces/INodeRegistry.sol)
- The address of the contract: listed in our [Official Document](https://docs.arpanetwork.io/randcast/supported-networks-and-parameters)

- Example calldata should look like this

  | Name                            | Type    | Data                                                                                                                                 |
  | ------------------------------- | ------- | ------------------------------------------------------------------------------------------------------------------------------------ |
  | assetAccountSignature.signature | bytes   | 0x444c3c9a6d487299162b02ac7e705ba533bc03445eda8d2e4f498bf430cbe21421a3c8933b0fa08e5fb43ee2a38896028694accebd0f77620eebf9bb93a3d4fc1c |
  | assetAccountSignature.salt      | bytes32 | 0x7fe94cad56d5aaeb5921b08ca90668654210fde42fed0c9507e8b5d796491bfc                                                                   |
  | assetAccountSignature.expiry    | uint256 | 1717429132                                                                                                                           |

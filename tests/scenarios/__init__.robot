*** Settings ***
Documentation       Node Registration Suite

Resource            src/common.resource
Resource            src/node.resource
Resource            src/contract.resource

Suite Setup         Set Enviorment And Deploy Contract
Suite Teardown      Teardown Scenario Testing Environment
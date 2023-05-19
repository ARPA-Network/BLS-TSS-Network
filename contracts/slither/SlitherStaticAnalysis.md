---
type: tech
keywords: 
tags: 
---

# Slither Static Analysis

Monday, October 10, 2022

## Links

[Foundry Slither](https://book.getfoundry.sh/config/static-analyzers)

[Slither Git Repo](https://github.com/crytic/slither)

[official git](https://github.com/crytic/slither)

[trail of bits blogpost](https://blog.trailofbits.com/2018/10/19/slither-a-solidity-static-analysis-framework/)

[slither wiki](https://github.com/crytic/slither/wiki)

## Installation

```bash

# use poetry here, learn it. 

cd contracts/tools
python3 -m venv .venv
. .venv/bin/activate
pip install -r requirements.txt
```

## Slither Usage

```bash
# Running slither
cd contracts
slither . # Run slither against all contracts
slither . --print human-summary # Print summary of findings
slither . --print contract-summary # Print summary of findings per contract
slither --json slither_output.json . # Output json

## exclude stuff
  --exclude-dependencies    Exclude results related to dependencies
  --exclude-optimization    Exclude optimization analyses
  --exclude-informational   Exclude informational impact analyses
  --exclude-low             Exclude low impact analyses
  --exclude-medium          Exclude medium impact analyses
  --exclude-high            Exclude high impact analyses


# Print inheritance graph
slither . --print inheritance-graph
xdot inheritance-graph.dot # Open graph in xdot
dot -Tpng inheritance-graph.dot -o inheritance-graph.png # Generate png
```

## Findings

Analysis of high findings [here](./findings.md)

## CLIther usage

CLIther is a CLI tool for analyzing slither output.

```bash
# Running clither
➜  slither git:(slitherTesting) ✗ python clither.py slither_output.json
                                                               
Loaded Slither Output: slither_output.json
Available Commands:
  - ls                        list finding summary
  - impact [impact]           list findings by impact [high|medium|low|informational|optimization]
  - detail [impact] [number]  display full findings details

Vulnerability / Remediation Info: https://github.com/crytic/slither/wiki/Detector-Documentation
> 
```

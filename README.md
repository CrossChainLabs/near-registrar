Top level account names (TLAs)
==============================

This repository includes TLA implementation in Rust (https://github.com/near/core-contracts/issues/25)

# Rust

_Using Gitpod? You can skip these setup steps!_

To run this project locally:

1. Prerequisites: Make sure you have Node.js ≥ 12 installed (https://nodejs.org), then use it to install [yarn]: `npm install --global yarn` (or just `npm i -g yarn`)
2. Install dependencies: `yarn install` (or just `yarn`)
3. Follow instructions for installing [rust] here https://docs.near.org/docs/roles/developer/contracts/near-sdk-rs#pre-requisites

Now you can run all the [rust]-related scripts listed in `package.json`! Scripts you might want to start with:

- `yarn test:unit:rs`: Runs all Rust tests in the project
- `yarn build:rs`: Compiles the Rust contracts to [Wasm] binaries

## Data collection

By using Gitpod in this project, you agree to opt-in to basic, anonymous analytics. No personal information is transmitted. Instead, these usage statistics aid in discovering potential bugs and user flow information.

  [rust]: https://www.rust-lang.org/
  [yarn]: https://yarnpkg.com/
  [AssemblyScript]: https://assemblyscript.org/
  [Wasm]: https://webassembly.org/

# Explorer

Explorer website.

## TOC

- [Explorer](#explorer)
  - [TOC](#toc)
  - [Build system and dev environment](#build-system-and-dev-environment)
  - [Environment variables](#environment-variables)
    - [NPM scripts](#npm-scripts)

## Build system and dev environment

The build system assumes that you have at least Node.js v20.x installed. The LTS version is 20.11.1 at the time of writing.

All terminal commands assume that you are positioned in root folder of the repository.
Run `npm install` from the root folder to get the necessary dependencies.

## Environment variables

The application defines these variables by reading a local `.env`

```
# *_PATH variables can be empty string, must start with a slash otherwise, must not end with a slash

VITE_BASE_PATH="" # Optional, set to '/explorer' when deploying to an 'apps.*' subdomain
VITE_BLOCKS_LIST_ENTRIES=100
VITE_CHAIN_INFO_ENTRIES=15
VITE_MARKET_DATA_REFETCH_INTERVAL=120000
VITE_NODE_URL="" # Optional, set to (e.g. 'https://nodes.dusk.network' to) override default
VITE_PROVISIONERS_REFETCH_INTERVAL=30000
VITE_REFETCH_INTERVAL=10000
VITE_RUSK_PATH="" # Optional, set to '/rusk' for dev mode
VITE_STATS_REFETCH_INTERVAL=1000
VITE_TRANSACTIONS_LIST_ENTRIES=100
VITE_FEATURE_TOKENS=true
VITE_FEATURE_BLOB_HASHES=true # requires node version >= 1.3.1-alpha.1
```

## Environment variables and dev mode

The application defaults to setting the node URL to `/`. In dev mode, requests made on `/rusk` are passed through a proxy to `localhost:8080`. When the app is running in dev mode, set `VITE_RUSK_PATH` to "/rusk".

The application will determine which network it is connected to by the subdomain it is hosted under, to override this and connect to any node set `VITE_NODE_URL`. Note that only `https://` protocol URLs are valid.

## NPM scripts

- `npm run build` generates the production build
- `npm run checks` runs all checks (lint, typecheck and test)
- `npm run dev` generates the development build and starts the dev server
- `npm run dev:host` generates the development build, starts the dev server and exposes it to the local network
- `npm run lint`: performs the linting checks
- `npm run lint:fix`: runs ESLint with the `--fix` flag to fix formatting errors
- `npm run preview` previews the production build
- `npm test` runs the test suite
- `npm run test:coverage` runs the test suite and generates the code coverage report in the `coverage` folder
- `npm run test:watch` runs the test suite in watch mode
- `npm run typecheck` runs the type checker
- `npm run typecheck:watch` runs the type checker in watch mode

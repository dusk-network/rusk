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
# can be empty string, must start with a slash otherwise, must not end with a slash
VITE_BASE_PATH=""
VITE_API_ENDPOINT="https://api.dusk.network/v1"
```

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

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

export class GraphQLError extends Error {
  constructor({ message, locations }) {
    super(message);
    this.locations = locations;
  }
}

export class GraphQLRequest extends Request {
  constructor(body, baseUrl) {
    const url = new URL("on/graphql/query/", baseUrl);
    super(url, { method: "POST", body });
  }

  async handle(response) {
    switch (response.status) {
      case 200:
        return await response.json();
      case 500:
        throw new GraphQLError((await response.json())[0]);
      default:
        throw new Error(
          `Unexpected [${response.status}] : ${response.statusText}}`,
        );
    }
  }
}

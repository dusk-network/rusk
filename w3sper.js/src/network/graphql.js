// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

export class GraphQLError extends Error {
  constructor({ message, locations, data, errors }) {
    super(message);
    this.locations = locations;
    this.data = data;
    this.errors = errors;
  }
}

const MISSING_QUERY_ERROR = "GraphQL query is required.";

function isGraphQLDocument(query) {
  return /^(query|mutation|subscription|fragment)\b|^\{/.test(query);
}

function createGraphQLError(payload) {
  return new GraphQLError({
    ...payload.errors[0],
    data: payload.data,
    errors: payload.errors,
  });
}

export function normalizeGraphQLRequest(request) {
  if (typeof request === "string") {
    const query = request.trim();

    if (!query) {
      throw new TypeError(MISSING_QUERY_ERROR);
    }

    return {
      query: isGraphQLDocument(query) ? query : `query { ${query} }`,
    };
  }

  const query = request?.query?.trim();
  if (!query) {
    throw new TypeError(MISSING_QUERY_ERROR);
  }

  return { ...request, query };
}

export function graphqlInit(payload, { headers, ...options } = {}) {
  headers = new Headers(headers);
  headers.set("Accept", "application/graphql-response+json, application/json");
  headers.set("Content-Type", "application/json");

  return {
    ...options,
    method: "POST",
    headers,
    body: JSON.stringify(payload),
  };
}

export async function parseGraphQLResponse(response) {
  const body = await response.text();
  let payload;

  if (body) {
    try {
      payload = JSON.parse(body);
    } catch {
      throw new Error("Invalid GraphQL response: non-JSON body");
    }
  }

  if (payload?.errors?.length) {
    throw createGraphQLError(payload);
  }

  if (response.status !== 200) {
    throw new Error(`Unexpected [${response.status}] : ${response.statusText}`);
  }

  if (payload && "data" in payload) {
    return payload.data;
  }

  throw new Error("Invalid GraphQL response: missing data/errors");
}

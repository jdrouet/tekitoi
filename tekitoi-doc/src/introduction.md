# Introduction

Tekitoi is a lightweight service that aggregates (or will) most of the oauth2 providers on the market into a single service.
The goal is to write the oauth2 service for all the providers once, in an efficient way, and make it available to everybody.

The "efficient" part is really important (to me) considering the current environment state. The goal is to make a service with
a minimal memory, CPU and energy footprint.

Although Tekitoi is not made to be a complete alternative to [Keycloack](https://www.keycloak.org/) (for now), the goal is
to have similar features, with a minimal footprint ([512Mo of RAM required for Keycloack](https://www.keycloak.org/docs/latest/server_installation/index.html#installation-prerequisites)
when Tekitoi consumes 2Mo of RAM).

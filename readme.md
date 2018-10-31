<!--
Copyright 2018 Fredrik Portström <https://portstrom.com>
This is free software distributed under the terms specified in
the file LICENSE at the top-level directory of this distribution.
-->

# Fetch Mediawiki site configuration

![Parse Wiki Text](https://portstrom.com/parse_wiki_text.svg)

Fetches the site configuration of a Mediawiki based wiki and outputs code for creating a configuration for [Parse Wiki Text](https://github.com/portstrom/parse_wiki_text) specific to that wiki. The domain name of the wiki is taken as a command line argument. The configuration is written to standard output. Only HTTPS connections are supported.

---
title: owui-lint
description: Documentation for the Open WebUI extension linter.
template: splash
hero:
  title: owui-lint
  tagline: Catch Open WebUI extension mistakes before they reach production.
  actions:
    - text: Get started
      link: overview/
      icon: right-arrow
    - text: GitHub
      link: https://github.com/christestet/owui-lint
      icon: external
---

import { Card, CardGrid } from '@astrojs/starlight/components';

<CardGrid>
  <Card title="Extension-aware checks" icon="setting">
    Validate Tools, Pipes, Filters, Actions, and Pipelines with rules derived from Open WebUI runtime behavior.
  </Card>
  <Card title="CI-ready output" icon="rocket">
    Emit text, JSON, GitHub annotation, or SARIF reports and tune exits with `--fail-on`.
  </Card>
  <Card title="Editor feedback" icon="pencil">
    Run `owui-lint server` for LSP diagnostics, hover details, completions, and quick fixes.
  </Card>
</CardGrid>

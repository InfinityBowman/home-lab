// @ts-check

import { tanstackConfig } from '@tanstack/eslint-config'

export default [
  { ignores: ['dist/**', 'routeTree.gen.ts'] },
  ...tanstackConfig,
]

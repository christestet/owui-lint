import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

export default defineConfig({
  site: 'https://christestet.github.io',
  base: '/owui-lint/',
  integrations: [
    starlight({
      title: 'owui-lint',
      description: 'A CLI linter for Open WebUI extensions.',
      favicon: 'owui-lint-icon.svg',
      logo: {
        src: './src/assets/owui-lint-icon.svg',
        alt: 'owui-lint',
      },
      social: [
        {
          icon: 'github',
          label: 'GitHub',
          href: 'https://github.com/christestet/owui-lint',
        },
      ],
      editLink: {
        baseUrl: 'https://github.com/christestet/owui-lint/edit/main/docs/',
      },
      sidebar: [
        {
          label: 'Guide',
          items: [
            { label: 'Overview', slug: 'overview' },
            { label: 'Install', slug: 'install' },
            { label: 'Usage', slug: 'usage' },
            { label: 'Configuration', slug: 'configuration' },
            { label: 'Editor Integration', slug: 'editor-integration' },
          ],
        },
        {
          label: 'Reference',
          items: [
            { label: 'CLI Reference', slug: 'reference/cli' },
            { label: 'Rules Reference', slug: 'reference/rules' },
          ],
        },
      ],
    }),
  ],
});

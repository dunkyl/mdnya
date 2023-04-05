export default {
  entry: './index.js',
  output: {
    filename: 'bundle.cjs',
  },
  experiments: {
    topLevelAwait: true,
  },
  target: 'node',
}
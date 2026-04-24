import { bumpVersion, readPackageVersion, syncVersionTargets, writePackageVersion } from './version-utils.mjs'

const increment = process.argv[2]?.trim()

if (!increment) {
  console.error('Usage: bun run scripts/bump-version.mjs <patch|minor|major|prepatch|preminor|premajor|prerelease>')
  process.exit(1)
}

const currentVersion = await readPackageVersion()
const nextVersion = bumpVersion(currentVersion, increment)

await writePackageVersion(nextVersion)
const touchedFiles = await syncVersionTargets(nextVersion)

console.log(`Bumped app version ${currentVersion} -> ${nextVersion}`)
if (touchedFiles.length) {
  console.log(`Synchronized: ${touchedFiles.join(', ')}`)
}

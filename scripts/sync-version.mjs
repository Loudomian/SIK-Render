import { readPackageVersion, syncVersionTargets } from './version-utils.mjs'

const version = await readPackageVersion()
const touchedFiles = await syncVersionTargets(version)

console.log(`Verified app version ${version}`)
if (touchedFiles.length) {
  console.log(`Synchronized: ${touchedFiles.join(', ')}`)
}

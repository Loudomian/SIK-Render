import { readFile, writeFile } from 'node:fs/promises'

const packageJsonPath = new URL('../package.json', import.meta.url)
const tauriConfigPath = new URL('../src-tauri/tauri.conf.json', import.meta.url)
const cargoTomlPath = new URL('../src-tauri/Cargo.toml', import.meta.url)
const cargoLockPath = new URL('../src-tauri/Cargo.lock', import.meta.url)

export const SEMVER_RE = /^(\d+)\.(\d+)\.(\d+)(?:-([0-9A-Za-z.-]+))?(?:\+([0-9A-Za-z.-]+))?$/

export function assertSemver(version) {
  if (!SEMVER_RE.test(version)) {
    throw new Error(`Invalid semver: ${version}`)
  }
}

export function bumpVersion(currentVersion, increment) {
  const match = currentVersion.match(SEMVER_RE)
  if (!match) {
    throw new Error(`Current package version is not valid semver: ${currentVersion}`)
  }

  let major = Number.parseInt(match[1], 10)
  let minor = Number.parseInt(match[2], 10)
  let patch = Number.parseInt(match[3], 10)
  const prerelease = match[4] ?? ''

  switch (increment) {
    case 'patch':
      return `${major}.${minor}.${patch + 1}`
    case 'minor':
      return `${major}.${minor + 1}.0`
    case 'major':
      return `${major + 1}.0.0`
    case 'prepatch':
      return `${major}.${minor}.${patch + 1}-rc.0`
    case 'preminor':
      return `${major}.${minor + 1}.0-rc.0`
    case 'premajor':
      return `${major + 1}.0.0-rc.0`
    case 'prerelease': {
      if (!prerelease) {
        return `${major}.${minor}.${patch + 1}-rc.0`
      }

      const segments = prerelease.split('.')
      const lastSegment = segments.at(-1) ?? ''
      if (/^\d+$/.test(lastSegment)) {
        segments[segments.length - 1] = String(Number.parseInt(lastSegment, 10) + 1)
        return `${major}.${minor}.${patch}-${segments.join('.')}`
      }
      return `${major}.${minor}.${patch}-${prerelease}.0`
    }
    default:
      throw new Error(`Unsupported increment: ${increment}`)
  }
}

export async function readPackageJson() {
  return JSON.parse(await readFile(packageJsonPath, 'utf8'))
}

export async function readPackageVersion() {
  const packageJson = await readPackageJson()
  assertSemver(packageJson.version)
  return packageJson.version
}

export async function writePackageVersion(version) {
  assertSemver(version)
  const packageJson = await readPackageJson()
  packageJson.version = version
  await writeFile(packageJsonPath, `${JSON.stringify(packageJson, null, 2)}\n`)
}

export async function syncVersionTargets(version) {
  assertSemver(version)
  const touchedFiles = []

  const tauriConfig = JSON.parse(await readFile(tauriConfigPath, 'utf8'))
  if (tauriConfig.version !== version) {
    tauriConfig.version = version
    await writeFile(tauriConfigPath, `${JSON.stringify(tauriConfig, null, 2)}\n`)
    touchedFiles.push('src-tauri/tauri.conf.json')
  }

  const cargoToml = await readFile(cargoTomlPath, 'utf8')
  const updatedCargoToml = cargoToml.replace(
    /(\[package\][\s\S]*?\nversion\s*=\s*")([^"]+)(")/,
    `$1${version}$3`,
  )
  if (updatedCargoToml !== cargoToml) {
    await writeFile(cargoTomlPath, updatedCargoToml)
    touchedFiles.push('src-tauri/Cargo.toml')
  }

  const cargoLock = await readFile(cargoLockPath, 'utf8')
  const updatedCargoLock = cargoLock.replace(
    /(\[\[package\]\]\s*\nname\s*=\s*"sik-render"\s*\nversion\s*=\s*")([^"]+)(")/,
    `$1${version}$3`,
  )
  if (updatedCargoLock !== cargoLock) {
    await writeFile(cargoLockPath, updatedCargoLock)
    touchedFiles.push('src-tauri/Cargo.lock')
  }

  return touchedFiles
}

import { execFile } from 'node:child_process'
import { readFile, writeFile } from 'node:fs/promises'
import { promisify } from 'node:util'

const execFileAsync = promisify(execFile)
const packageJsonPath = new URL('../package.json', import.meta.url)
const tauriConfigPath = new URL('../src-tauri/tauri.conf.json', import.meta.url)
const cargoTomlPath = new URL('../src-tauri/Cargo.toml', import.meta.url)
const cargoLockPath = new URL('../src-tauri/Cargo.lock', import.meta.url)
const SEMVER_RE = /^(\d+)\.(\d+)\.(\d+)(?:-([0-9A-Za-z.-]+))?(?:\+([0-9A-Za-z.-]+))?$/
const releaseFiles = ['package.json', 'src-tauri/tauri.conf.json', 'src-tauri/Cargo.toml', 'src-tauri/Cargo.lock']

const command = process.argv[2]?.trim()
const increment = process.argv[3]?.trim()

function usage() {
  console.error('Usage:')
  console.error('  bun run version:sync')
  console.error('  bun run version:bump <patch|minor|major|prepatch|preminor|premajor|prerelease>')
  console.error('  bun run version:release <patch|minor|major|prepatch|preminor|premajor|prerelease>')
}

function assertSemver(version) {
  if (!SEMVER_RE.test(version)) {
    throw new Error(`Invalid semver: ${version}`)
  }
}

function bumpVersion(currentVersion, type) {
  const match = currentVersion.match(SEMVER_RE)
  if (!match) {
    throw new Error(`Current package version is not valid semver: ${currentVersion}`)
  }

  const major = Number.parseInt(match[1], 10)
  const minor = Number.parseInt(match[2], 10)
  const patch = Number.parseInt(match[3], 10)
  const prerelease = match[4] ?? ''

  switch (type) {
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
      if (!prerelease) return `${major}.${minor}.${patch + 1}-rc.0`

      const segments = prerelease.split('.')
      const lastSegment = segments.at(-1) ?? ''
      if (/^\d+$/.test(lastSegment)) {
        segments[segments.length - 1] = String(Number.parseInt(lastSegment, 10) + 1)
        return `${major}.${minor}.${patch}-${segments.join('.')}`
      }
      return `${major}.${minor}.${patch}-${prerelease}.0`
    }
    default:
      throw new Error(`Unsupported increment: ${type}`)
  }
}

async function git(args) {
  const { stdout } = await execFileAsync('git', args)
  return stdout.trim()
}

async function readPackageJson() {
  return JSON.parse(await readFile(packageJsonPath, 'utf8'))
}

async function readPackageVersion() {
  const packageJson = await readPackageJson()
  assertSemver(packageJson.version)
  return packageJson.version
}

async function writePackageVersion(version) {
  assertSemver(version)
  const packageJson = await readPackageJson()
  packageJson.version = version
  await writeFile(packageJsonPath, `${JSON.stringify(packageJson, null, 2)}\n`)
}

async function syncVersionTargets(version) {
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

async function assertCleanWorktree() {
  const status = await git(['status', '--porcelain'])
  if (status) {
    throw new Error('Release requires a clean working tree. Commit or stash current changes first.')
  }
}

async function sync() {
  const version = await readPackageVersion()
  const touchedFiles = await syncVersionTargets(version)
  console.log(`Verified app version ${version}`)
  if (touchedFiles.length) console.log(`Synchronized: ${touchedFiles.join(', ')}`)
}

async function bump(type) {
  if (!type) throw new Error('Missing increment.')
  const currentVersion = await readPackageVersion()
  const nextVersion = bumpVersion(currentVersion, type)
  await writePackageVersion(nextVersion)
  const touchedFiles = await syncVersionTargets(nextVersion)
  console.log(`Bumped app version ${currentVersion} -> ${nextVersion}`)
  if (touchedFiles.length) console.log(`Synchronized: ${touchedFiles.join(', ')}`)
  return nextVersion
}

async function release(type) {
  await assertCleanWorktree()
  const nextVersion = await bump(type)
  const tagName = `v${nextVersion}`

  if (await git(['tag', '--list', tagName])) {
    throw new Error(`Tag already exists: ${tagName}`)
  }

  await git(['add', ...releaseFiles])
  await git(['commit', '-m', `chore: release ${tagName}`])
  await git(['tag', tagName])

  console.log(`Created commit and tag: ${tagName}`)
  console.log(`Push with: git push && git push origin ${tagName}`)
}

try {
  if (command === 'sync') {
    await sync()
  } else if (command === 'bump') {
    await bump(increment)
  } else if (command === 'release') {
    await release(increment)
  } else {
    usage()
    process.exit(1)
  }
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error))
  process.exit(1)
}

# CI/CD Pipeline Quick Reference

## Status Badges

[![Build and Test](https://github.com/BleepEcosystem/BLEEP-V1/actions/workflows/build-and-test.yml/badge.svg?branch=main)](https://github.com/BleepEcosystem/BLEEP-V1/actions/workflows/build-and-test.yml)
[![Code Quality](https://github.com/BleepEcosystem/BLEEP-V1/actions/workflows/code-quality.yml/badge.svg?branch=main)](https://github.com/BleepEcosystem/BLEEP-V1/actions/workflows/code-quality.yml)
[![Security](https://github.com/BleepEcosystem/BLEEP-V1/actions/workflows/security.yml/badge.svg?branch=main)](https://github.com/BleepEcosystem/BLEEP-V1/actions/workflows/security.yml)
[![Docker Build](https://github.com/BleepEcosystem/BLEEP-V1/actions/workflows/docker.yml/badge.svg?branch=main)](https://github.com/BleepEcosystem/BLEEP-V1/actions/workflows/docker.yml)
[![Performance](https://github.com/BleepEcosystem/BLEEP-V1/actions/workflows/performance.yml/badge.svg?branch=main)](https://github.com/BleepEcosystem/BLEEP-V1/actions/workflows/performance.yml)

## Setup

### One-time Setup

```bash
# Initialize development environment
chmod +x scripts/setup-cicd.sh
./scripts/setup-cicd.sh
```

### Configure GitHub Secrets

```bash
# Using GitHub CLI
gh secret set CARGO_TOKEN --body '<your-token>'
gh secret set DOCKERHUB_USERNAME --body '<username>'
gh secret set DOCKERHUB_TOKEN --body '<token>'
gh secret set GITGUARDIAN_API_KEY --body '<key>'
```

## Common Commands

### Build & Test

| Command | Purpose |
|---------|---------|
| `make build` | Build project (debug mode) |
| `make build-release` | Build project (optimized) |
| `make test` | Run all tests |
| `make test-unit` | Run unit tests |
| `make test-integration` | Run integration tests |

### Code Quality

| Command | Purpose |
|---------|---------|
| `make fmt` | Auto-format code |
| `make fmt-check` | Check formatting without fixing |
| `make clippy` | Run clippy linter |
| `make lint` | Run all linters (fmt + clippy) |
| `make coverage` | Generate coverage report |
| `make doc` | Generate documentation |

### Performance

| Command | Purpose |
|---------|---------|
| `make bench` | Run all benchmarks |
| `make bench-consensus` | Consensus benchmarks |
| `make bench-crypto` | Crypto benchmarks |
| `make profile-memory` | Memory profiling |

### Docker

| Command | Purpose |
|---------|---------|
| `make docker-build` | Build Docker image |
| `make docker-run` | Run single node |
| `make docker-compose-up` | Start testnet (4 validators) |
| `make docker-compose-down` | Stop testnet |
| `make docker-compose-logs` | View logs |

### Development

| Command | Purpose |
|---------|---------|
| `make watch` | Auto-rebuild on changes |
| `make install` | Install binaries locally |
| `make security` | Run all security checks |
| `make ci-local` | Run full local CI (like GitHub Actions) |
| `make clean` | Clean build artifacts |

## Workflow Triggers

### Manual Trigger

```bash
# Using GitHub CLI
gh workflow run build-and-test.yml
gh workflow run code-quality.yml
gh workflow run security.yml
gh workflow run docker.yml
gh workflow run performance.yml
```

### Automatic Triggers

| Workflow | Trigger |
|----------|---------|
| `build-and-test` | Push, PR, manual |
| `code-quality` | Push, PR, manual |
| `security` | Push, PR, weekly, manual |
| `docker` | Push, tags, PR, manual |
| `performance` | Push to main, PR, manual |
| `release` | Tag creation, manual |
| `nightly` | Daily 2 AM UTC, manual |

## Understanding Workflows

### Build and Test Pipeline
1. **Build** - Compiles on Ubuntu (stable & release) and macOS
2. **Test** - Unit, doc, integration tests + coverage
3. **Analysis** - Memory (valgrind), behavior (miri), sanitizers

### Security Pipeline
1. **Audit** - Dependency vulnerabilities (cargo-audit)
2. **License** - License compliance (cargo-deny)
3. **SBOM** - Software bill of materials
4. **Scanning** - Trivy, Semgrep, CodeQL
5. **Secrets** - GitGuardian detection

### Docker Pipeline
1. **Build** - Multi-architecture images (amd64, arm64, armv7)
2. **Security** - Container vulnerability scanning
3. **Push** - Publish to GHCR and Docker Hub (if configured)

### Performance Pipeline
1. **Benchmarks** - Consensus, crypto, VM, P2P
2. **Stress Tests** - Multi-threaded load testing
3. **Profiling** - Memory and CPU analysis
4. **Comparison** - PR vs baseline comparison

## Monitoring CI/CD

### View Workflow Status

```bash
# List all workflows
gh workflow list

# View recent runs
gh run list --repo BleepEcosystem/BLEEP-V1

# View specific run details
gh run view <RUN_ID> --log

# Check workflow status
gh api repos/BleepEcosystem/BLEEP-V1/actions/runs | jq
```

### View Metrics

- **GitHub Actions**: https://github.com/BleepEcosystem/BLEEP-V1/actions
- **Build Time**: Check individual workflow logs
- **Test Coverage**: Codecov dashboard (after setup)
- **Dependencies**: Dependabot alerts page

## Troubleshooting

### Workflow Failures

```bash
# Re-run failed workflows
gh run rerun <RUN_ID>

# View detailed logs
gh run view <RUN_ID> --log | less

# Check specific job
gh run view <RUN_ID> --log --job-id <JOB_ID>
```

### Local Troubleshooting

```bash
# Reproduce CI environment locally
make ci-local

# Check Rust version
rustc --version
cargo --version

# Verify all linters
cargo fmt --all -- --check
cargo clippy --all-targets --all-features
cargo test --workspace

# Generate coverage locally
make coverage
```

### Common Issues

**Build Timeout**
- Check for infinite loops
- Profile memory: `make profile-memory`
- Increase timeout in workflow

**Test Failures**
- Run locally: `cargo test --workspace`
- Check output: `RUST_BACKTRACE=1 cargo test`
- Profile: `make profile-cpu`

**Docker Build Fails**
- Build locally: `docker build -t bleep:test .`
- Check disk: `docker system prune`
- View logs: `docker build --progress=plain .`

## Release Process

### Create Release

```bash
# Tag release
git tag v1.0.0
git push origin v1.0.0

# Workflow automatically:
# 1. Creates GitHub Release
# 2. Builds binaries (all platforms)
# 3. Publishes to crates.io
# 4. Pushes Docker images
# 5. Deploys documentation
```

### Manual Release

```bash
# Trigger release workflow
gh workflow run release.yml

# Check release status
gh release list
```

## Performance Optimization

### Build Time

- Incremental compilation enabled
- Workspace caching active
- Link-time optimization in release mode
- Parallel build across cores

### Test Execution

- Parallel test runs (adjust with `RUST_TEST_THREADS`)
- Separate unit/integration test batches
- Doc test compilation separate

### CI/CD Duration

- Average build: 5-10 minutes
- Average tests: 8-15 minutes
- Average all checks: 20-30 minutes

## Advanced Usage

### Custom Build Features

```bash
# Build with specific features
cargo build --features "feature1,feature2"

# Build all features
cargo build --all-features

# No default features
cargo build --no-default-features
```

### Workflow Customization

Edit `.github/workflows/*.yml`:
- Adjust build matrix
- Change trigger conditions
- Add custom jobs
- Modify timeout settings

### Local CI Simulation

```bash
# Docker-based CI simulation
docker run -v $(pwd):/app -w /app rust:latest \
  bash -c "cargo build && cargo test && cargo clippy"
```

## Security Best Practices

1. **Never commit secrets** - Use GitHub Secrets
2. **Rotate tokens regularly** - Monthly updates
3. **Review dependencies** - Dependabot PRs
4. **Monitor advisories** - Weekly security checks
5. **Sign commits** - Enable require signed commits

## Resources

- 📖 [Full CI/CD Documentation](CI_CD_PIPELINE.md)
- 📦 [Deployment Guide](DEPLOYMENT.md)
- 🔧 [Makefile Targets](../Makefile)
- 📋 [Workflow Files](../.github/workflows/)
- 🐳 [Docker Configuration](../docker-compose.yml)

## Support

For issues or improvements:
1. Check [GitHub Issues](https://github.com/BleepEcosystem/BLEEP-V1/issues)
2. Review documentation
3. Test locally with `make ci-local`
4. Create detailed issue report
5. Submit PR with fix

---

**Last Updated**: 2024
**Maintainers**: BLEEP Team

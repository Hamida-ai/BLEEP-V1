# BLEEP CI/CD Pipeline Documentation

## Overview

BLEEP uses a comprehensive, production-grade CI/CD pipeline built on GitHub Actions. The pipeline automates:

- **Building** across multiple platforms and architectures
- **Testing** with extensive coverage and various test types
- **Code Quality** checks (linting, formatting, complexity)
- **Security** scanning (dependency vulnerabilities, code analysis)
- **Performance** benchmarking and profiling
- **Docker** image building and multi-arch support
- **Release** management and artifact publishing
- **Documentation** generation and deployment

---

## Workflows

### 1. Build and Test (`build-and-test.yml`)

**Trigger:** Push to `main`/`develop`, PRs, manual dispatch

**Jobs:**
- **Build**: Compiles on Ubuntu and macOS (stable Rust)
- **Test**: Unit, doc, and integration tests
- **Coverage**: Code coverage measurement with tarpaulin
- **Miri**: Detects undefined behavior (nightly)
- **Valgrind**: Memory safety checks
- **Sanitizers**: AddressSanitizer, MemorySanitizer, ThreadSanitizer

**Key Features:**
- Parallel execution across matrix strategies
- Caching Rust artifacts for speed
- All-features compilation testing

---

### 2. Code Quality (`code-quality.yml`)

**Trigger:** Push to `main`/`develop`, PRs, manual dispatch

**Jobs:**
- **fmt**: Check code formatting with rustfmt
- **clippy**: Lint with strict warnings
- **doc**: Generate documentation with link checking
- **deadcode**: Detect unused code (nightly)
- **unused-deps**: Find unused dependencies (nightly)
- **complexity**: Analyze code metrics
- **bench-check**: Ensure benchmarks compile
- **toml-check**: Validate TOML and JSON configs
- **audit-advisories**: Check advisory database

**Quality Gates:**
- All warnings must pass clippy checks
- Code must be properly formatted
- Documentation must compile
- Config files must be valid

---

### 3. Security (`security.yml`)

**Trigger:** Push, PRs, weekly schedule, manual dispatch

**Jobs:**
- **cargo-audit**: Dependency vulnerability scanning
- **cargo-deny**: License and advisory policy checking
- **trivy**: Container and filesystem scanning (SARIF upload)
- **semgrep**: Static analysis with security patterns
- **codeql**: GitHub's CodeQL analysis engine
- **supply-chain**: SBOM generation and locked deps check
- **secret-scan**: GitGuardian secret detection
- **fuzzing**: AFL infrastructure availability
- **license-check**: License compliance verification

**Security Gates:**
- Zero known vulnerabilities in dependencies
- Licenses compatible with project policy
- No secrets exposed in code

---

### 4. Docker (`docker.yml`)

**Trigger:** Push to branches/tags, PRs, manual dispatch

**Jobs:**
- **build**: Standard x86_64 image
- **validate**: Container runtime validation
- **security-scan**: Trivy container scanning
- **build-multiarch**: Multi-architecture builds (amd64, arm64, armv7)

**Features:**
- Multi-stage Docker build for optimization
- Container registry push to GHCR
- SHA256 image verification
- Health checks included
- Docker Buildx with caching

**Image Tags:**
```
ghcr.io/bleepecosystem/bleep-v1:latest
ghcr.io/bleepecosystem/bleep-v1:v1.0.0
ghcr.io/bleepecosystem/bleep-v1:sha-abc123
ghcr.io/bleepecosystem/bleep-v1:main
```

---

### 5. Performance (`performance.yml`)

**Trigger:** Push to `main`, PRs, manual dispatch

**Jobs:**
- **benchmark**: Full benchmark suite
- **stress-test**: Multi-threaded stress testing
- **memory-profiling**: Memory usage analysis
- **throughput-test**: Example-based throughput
- **latency-test**: Consensus latency measurement
- **load-test**: High-load scenario testing
- **benchmark-comparison**: Compare PR vs baseline

**Metrics Tracked:**
- Consensus throughput (TPS)
- Block finalization latency
- Memory usage patterns
- CPU utilization
- P2P message throughput

---

### 6. Release (`release.yml`)

**Trigger:** Tag push (`v*`), manual dispatch

**Jobs:**
- **create-release**: Creates GitHub Release with changelog
- **build-artifacts**: Multi-platform binary builds
  - Linux x86_64 (GNU)
  - Linux ARM64
  - macOS x86_64 (Intel)
  - macOS ARM64 (Apple Silicon)
  - Windows x86_64 (MSVC)
- **publish-cargo**: Publish crates to crates.io
- **publish-docker**: Push images to registries
- **github-pages**: Deploy API docs to GitHub Pages
- **notify**: Release summary notification

**Artifacts:**
- Native binaries for each platform
- SHA256 checksums
- Docker images with version tags
- API documentation

---

### 7. Nightly (`nightly.yml`)

**Trigger:** Daily 2 AM UTC, manual dispatch

**Jobs:**
- **nightly-test**: Full test suite on nightly Rust
- **nightly-miri**: Undefined behavior detection
- **nightly-bench**: Performance benchmarks
- **dependency-check**: Outdated dependencies check
- **minimal-versions**: Test with minimum dependency versions
- **clippy-nightly**: Lint with nightly features

---

## Local Development

### Quick Start

```bash
# Setup development environment
make dev-setup

# Common workflows
make lint        # Format + clippy
make test        # All tests
make build-release  # Optimized build
make coverage    # Generate coverage report
make bench       # Run benchmarks
make doc         # Generate documentation
make ci-local    # Run all local CI checks
```

### Watch Mode

```bash
make watch  # Auto-rebuild and test on file changes
```

### Docker Development

```bash
# Single node
make docker-run

# Multi-node testnet (4 validators + 1 full node)
make docker-compose-up
make docker-compose-logs
make docker-compose-down

# Services available:
# - Validators:  http://localhost:8001-8004
# - Full Node:   http://localhost:8005
# - Prometheus:  http://localhost:9090
# - Grafana:     http://localhost:3000 (admin/admin)
```

---

## Configuration

### GitHub Secrets Required

For full CI/CD functionality, configure these secrets in repository settings:

| Secret | Purpose | Location |
|--------|---------|----------|
| `GITHUB_TOKEN` | Auto-included | GitHub provides |
| `CARGO_TOKEN` | Publish to crates.io | crates.io settings |
| `DOCKERHUB_USERNAME` | Docker Hub push | Docker Hub account |
| `DOCKERHUB_TOKEN` | Docker Hub auth | Docker Hub settings |
| `GITGUARDIAN_API_KEY` | Secret scanning | GitGuardian dashboard |

### Workflow Customization

Edit workflow files in `.github/workflows/` to:
- Adjust build matrix (Python versions, Rust versions, etc.)
- Modify trigger conditions
- Add/remove jobs
- Change schedule times

---

## Monitoring & Observability

### Metrics

The pipeline generates metrics available in:
- **GitHub Actions**: Real-time logs and summaries
- **Prometheus**: Container metrics (port 9090)
- **Grafana**: Visualization dashboard (port 3000)
- **Codecov**: Code coverage trends

### Accessing Logs

```bash
# View workflow logs
gh run list --repo BleepEcosystem/BLEEP-V1
gh run view <RUN_ID>

# Docker Compose services
make docker-compose-logs
docker-compose logs -f validator-1
```

---

## Best Practices

### For Contributors

1. **Run local checks before pushing:**
   ```bash
   make fmt         # Auto-format code
   make clippy      # Find issues
   make test        # Run tests
   ```

2. **Keep commits atomic** - one logical change per commit

3. **Use descriptive commit messages:**
   ```
   feat: add consensus layer correction
   fix: resolve block validation deadlock
   docs: update deployment guide
   ```

4. **Add tests for new features:**
   ```bash
   cargo test --package <crate> --lib
   ```

5. **Check PR template** - all sections mandatory

### For Reviewers

- Check CI/CD status - all checks must pass
- Review code coverage - aim for >80%
- Verify security checks - no vulnerabilities
- Test functionality locally if needed
- Approve after all concerns addressed

### For Maintainers

1. **Regular updates:**
   - Update Rust toolchain weekly
   - Review dependency updates via Dependabot
   - Monitor security advisories

2. **Release process:**
   ```bash
   git tag v1.0.0
   git push origin v1.0.0
   # Workflow handles builds, tests, publishing
   ```

3. **Monitor pipeline health:**
   - Check action run times for regressions
   - Update workflows for API changes
   - Maintain coverage standards

---

## Troubleshooting

### Common Issues

**Build Failure**
```bash
# Clean build locally
make clean
make build-release

# Check Rust version
rustc --version

# Update toolchain
rustup update stable
```

**Test Timeout**
- Check for infinite loops
- Profile memory usage: `make profile-memory`
- Increase timeout in workflow if legitimate

**Docker Build Failure**
```bash
# Build locally
docker build -t bleep:test .

# Check disk space
docker system prune
```

**Coverage Below Threshold**
```bash
# Generate local coverage report
make coverage

# Check uncovered code
cargo tarpaulin --out Html --exclude-files tests/*
```

**Dependency Vulnerability**
```bash
# Check advisories
cargo audit

# Update problematic crate
cargo update <crate>

# Or pin secure version in Cargo.toml
```

---

## Performance Optimization

### Build Times

- Enables incremental compilation
- Uses workspace caching
- Parallel compilation across CPU cores
- Link-time optimization in release mode

### Test Parallel Execution

Default parallel test count: CPU count
- Adjust via `RUST_TEST_THREADS` environment variable

### Docker Build Optimization

- Multi-stage builds reduce final image size
- Layer caching speeds up rebuilds
- `.dockerignore` excludes unnecessary files

---

## Security Considerations

1. **Secrets Management**
   - Never commit `.env` files
   - Use GitHub Secrets for sensitive values
   - Rotate tokens regularly

2. **Dependency Safety**
   - Weekly dependency audits
   - Automated Dependabot updates
   - SBOM generation for supply chain

3. **Code Scanning**
   - Trivy for container vulnerabilities
   - CodeQL for code patterns
   - Semgrep for security issues
   - Miri for undefined behavior

---

## Advanced Usage

### Matrix Builds

Customize build matrix in workflow files:

```yaml
strategy:
  matrix:
    rust: [stable, nightly, 1.70.0]
    os: [ubuntu-latest, macos-latest, windows-latest]
```

### Conditional Job Execution

```yaml
if: github.event_name == 'push' && startsWith(github.ref, 'refs/tags/')
```

### Custom Caching

```yaml
- uses: Swatinem/rust-cache@v2
  with:
    cache-all-crates: true
    key: ${{ matrix.target }}
```

---

## Support & Contributing

For CI/CD issues or improvements:

1. Check existing GitHub Issues
2. Review this documentation
3. Test changes locally first
4. Create PR with small, focused changes
5. Reference related issues/PRs

---

## Further Reading

- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [Rust Build Performance](https://doc.rust-lang.org/cargo/profiles/)
- [Docker Best Practices](https://docs.docker.com/develop/dev-best-practices/)
- [BLEEP Architecture](../PHASE5_COMPLETE_ARCHITECTURE.md)

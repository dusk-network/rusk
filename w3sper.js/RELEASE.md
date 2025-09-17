# W3sper.js Release Instructions

This document outlines the standard process for releasing updates to the w3sper.js package to JSR (JavaScript Registry) following semantic versioning principles.

## Example: W3sper.js v1.2.0 Release

### 1. Create a Release Issue

- Create a new issue in the project repository
- Title: `w3sper.js: Release v1.2.0`
- Add appropriate labels (e.g. module: `w3sper.js`, type: `task`)

### 2. Create a Release Branch

- Create a branch from the latest `main`
- Name the branch using the issue number (e.g., `feature-ABC-123`)
- **Note:** Do not use version numbers for branch names to avoid conflicts with tags

```bash
git checkout main
git pull
git checkout -b feature-ABC-123
```

### 3. Update Version Number

Update the version in `deno.json` according to semantic versioning:

- **Patch** (e.g., 1.1.0 → 1.1.1): for bug fixes and minor changes
- **Minor** (e.g., 1.1.0 → 1.2.0): for new features, backward compatible
- **Major** (e.g., 1.1.0 → 2.0.0): for breaking changes

For this example (minor version bump):

```json
{
  "name": "@dusk/w3sper",
  "version": "1.2.0",
  ...
}
```

### 4. Update the CHANGELOG.md

- Add a new section for the version being released
- Copy relevant changes from "Unreleased" section to the new version section
- Keep the "Unreleased" section with any features that remain hidden to users
- Ensure the format follows the project's changelog conventions
- Include issue references in brackets (e.g., [#1234])

Example CHANGELOG.md format:

```markdown
# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Added

### Changed

### Removed

### Fixed

## [1.2.0] - 2025-06-19

### Added

- New transaction filtering capabilities [#1234]
- Support for additional wallet operations [#1235]

### Changed

- Improved WebSocket connection handling [#1236]
- Enhanced error messages for better debugging [#1237]

### Fixed

- Fixed memory leak in event listeners [#1238]
- Resolved connection timeout issues [#1239]

## [1.1.0] - 2025-03-26

...

<!-- ISSUES -->
```

### 5. Validate Package Configuration

Ensure the `deno.json` file is properly configured for JSR publishing:

- Verify the `name` field follows JSR naming conventions (`@dusk/w3sper`)
- Check that `exports` points to the correct entry file
- Confirm `publish.include` contains all necessary files
- Verify `publish.exclude` omits development files

### 6. Run Tests and Validation

```bash
# Run the test suite
deno task test

# Validate the package can be imported
deno run --allow-net --allow-read src/mod.js

# Check for any linting issues
deno lint
```

### 7. Commit, dry-run and push the changes

```bash
git add .
git commit -m "w3sper.js: Release v1.2.0"

# Validate JSR package structure
deno publish --dry-run

git push origin feature-ABC-123
```

### 8. Create and Review Pull Request

- Open a PR targeting `main`
- Copy the changelog section for the new version into the PR description
- Request code review from team members

```bash
git add CHANGELOG.md deno.json
git commit -m "w3sper.js: Release v1.2.0"
git push origin feature-ABC-123
```

### 9. Create Annotated Tag

**Option A: Tag from branch (after approval)**

```bash
git tag -a w3sper-v1.2.0 -m "w3sper.js: Release v1.2.0"
```

**Option B: Merge to main then tag (recommended)**

```bash
git checkout main
git pull
git tag -a w3sper-v1.2.0 -m "w3sper.js: Release v1.2.0"
```

**Important note:** Ensure the tag name (`w3sper-v1.2.0`) matches exactly with the entry added to the CHANGELOG.md in step 4.

### 10. Push the Tag

```bash
git push origin w3sper-v1.2.0
```

### 11. Publish to JSR

Publish the package to the JavaScript Registry:

```bash
# Ensure you're on the tagged commit
git checkout w3sper-v1.2.0

# Navigate to the w3sper.js directory
cd w3sper.js

# Publish to JSR (requires authentication)
deno publish
```

**Note:** Ensure you have the necessary permissions to publish to the `@dusk` scope on JSR.

### 12. Verify JSR Publication

- Check that the package appears on JSR: https://jsr.io/@dusk/w3sper
- Verify the version number matches the release
- Test importing the package from JSR:

```bash
deno run --allow-net -e "import { /* your exports */ } from 'jsr:@dusk/w3sper@1.2.0'; console.log('Import successful');"
```

### 13. Update Documentation

- Verify that README.md examples use the correct version
- Update any documentation that references version-specific features
- Check that JSR package page displays correctly

### 14. Integration Testing

Test the published package in real-world scenarios:

- Create a simple test project that imports from JSR
- Verify all exported functions work as expected
- Test in different Deno runtime environments if applicable

### 15. Announce the Release

- Notify development team about the new release
- Update any dependent projects that use w3sper.js
- Consider posting in relevant Discord channels or communication platforms
- Update project documentation that references w3sper.js version requirements

## JSR-Specific Considerations

### Package Naming

- Follow JSR scoped package naming: `@dusk/w3sper`
- Ensure the scope (`@dusk`) is properly registered and accessible

### Version Management

- JSR uses the version field in `deno.json` as the source of truth
- Once published, a version cannot be changed or deleted
- Use pre-release versions (e.g., `1.2.0-beta.1`) for testing

### Publishing Permissions

- Ensure you have publish permissions for the `@dusk` scope
- Consider setting up automated publishing via CI/CD for consistency

### Documentation

- JSR automatically generates documentation from JSDoc comments
- Ensure all public APIs are properly documented
- Include usage examples in README.md

## Rollback Procedure (If Needed)

If critical issues are discovered after JSR publication:

1. **Cannot unpublish from JSR** - versions are immutable
2. Create a hotfix branch from the problematic tag
3. Fix the issue and release a new patch version immediately
4. Update documentation to recommend the new version
5. Communicate transparently about the issue and resolution

## Release Checklist

- [ ] Create issue for release
- [ ] Create feature branch with issue number
- [ ] Update version in deno.json
- [ ] Update CHANGELOG.md with new version
- [ ] Run tests and validation (`deno task test`)
- [ ] Validate package structure (`deno publish --dry-run`)
- [ ] Open and get approval on PR
- [ ] Create and push annotated tag
- [ ] Publish to JSR (`deno publish`)
- [ ] Verify JSR publication and package page
- [ ] Test importing from JSR
- [ ] Integration testing with dependent projects
- [ ] Update documentation if needed
- [ ] Announce release to stakeholders

## Troubleshooting

### Common JSR Publishing Issues

1. **Authentication Error**

   ```bash
   deno auth https://jsr.io
   ```

2. **Scope Permission Issues**

   - Verify you have publish rights to `@dusk` scope
   - Contact scope administrators if needed

3. **Package Validation Errors**

   - Run `deno publish --dry-run` to identify issues
   - Check `deno.json` configuration
   - Ensure all exported modules are valid

4. **Version Conflicts**
   - JSR versions are immutable
   - Increment version number if the same version exists
   - Use semantic versioning strictly

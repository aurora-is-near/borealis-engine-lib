import os
import re
import datetime
from pathlib import Path

REPO_URL = "https://github.com/aurora-is-near/borealis-engine-lib"

VERSION_PATTERN = r'^[\d\.]+\-[\d\.]+(\-[a-z]+\.\d+)?$'
UNRELEASED_SECTION_PATTERN = r'^## \[Unreleased\]'
RELEASE_ENTRY_PATTERN = r'^## \[([\d\.\-a-z]+)\]'
CHANGES_SECTION_PATTERN = r'#+\s+Changes'
PR_LINK_PATTERN = r'\[#\d+\]: ' + re.escape(REPO_URL) + r'/pull/\d+'

def validate_version(version):
    """Validate that the version format is correct."""
    if not version:
        return False

    valid_version_pattern = re.compile(VERSION_PATTERN)
    return bool(valid_version_pattern.match(version))

def get_date_and_path():
    """Get current UTC date and changelog path."""
    current_date = datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%d")
    custom_path = os.environ.get("CHANGELOG_PATH")
    changelog_path = Path(custom_path) if custom_path else Path("CHANGES.md")

    if not changelog_path.exists():
        print(f"Error: {changelog_path} not found")
        exit(1)

    return current_date, changelog_path

def extract_nearcore_version(version):
    """Extract nearcore version from the full version string."""
    return '-'.join(version.split('-')[1:]) if '-' in version else version

def split_changelog_content(content):
    """Split the changelog into upper content and link sections."""
    link_section_pattern = re.compile(r'^(?=\[Unreleased\]:)', re.MULTILINE)
    match = link_section_pattern.search(content)

    if not match:
        print("Error: Could not find the link definitions section in CHANGES.md")
        exit(1)

    return content[:match.start()], content[match.start():]

def find_unreleased_section(upper_section):
    """Find the unreleased section in the changelog."""
    unreleased_pattern = re.compile(UNRELEASED_SECTION_PATTERN, re.MULTILINE)
    unreleased_match = unreleased_pattern.search(upper_section)

    if not unreleased_match:
        print("Error: Could not find [Unreleased] section in CHANGES.md")
        exit(1)

    return unreleased_match

def find_previous_version(upper_section, unreleased_match):
    """Find the previous version in the changelog."""
    release_pattern = re.compile(RELEASE_ENTRY_PATTERN, re.MULTILINE)
    releases = release_pattern.finditer(upper_section, unreleased_match.end())

    try:
        first_release = next(releases)
        prev_version = first_release.group(1)
        unreleased_section_end = first_release.start()
        unreleased_content = upper_section[unreleased_match.end():unreleased_section_end].strip()
        return prev_version, unreleased_section_end, unreleased_content
    except StopIteration:
        print("Error: Could not find any previous release entries")
        exit(1)

def create_new_entry(unreleased_content, new_entry):
    """Create the new entry with appropriate headers."""
    if not unreleased_content:
        return f"### Changes\n\n{new_entry}"

    # Check if there's a Changes section with any heading level
    changes_section_pattern = re.compile(CHANGES_SECTION_PATTERN, re.MULTILINE)
    changes_match = changes_section_pattern.search(unreleased_content)

    if changes_match:
        changes_end = changes_match.end()
        # Insert new entry after the "Changes" line
        return (
            unreleased_content[:changes_end] +
            "\n\n" + new_entry + "\n" +
            unreleased_content[changes_end:].lstrip()
        )
    else:
        # If no Changes section, add one with the new entry
        return f"### Changes\n\n{new_entry}\n\n{unreleased_content}"

def add_pr_link(content, pr_link):
    """Add PR link to the content."""
    pr_link_pattern = re.compile(PR_LINK_PATTERN)
    pr_links_exist = pr_link_pattern.search(content)

    if pr_links_exist:
        return content + f"\n{pr_link}"
    else:
        return content + f"\n\n{pr_link}"

def update_link_definitions(bottom_section, new_version, prev_version):
    """Update link definitions in bottom section."""
    bottom_section = re.sub(
        r'\[Unreleased\]: .*',
        f'[Unreleased]: {REPO_URL}/{new_version}...main',
        bottom_section
    )

    new_link_def = f'[{new_version}]: {REPO_URL}/compare/{prev_version}...{new_version}'

    return re.sub(
        r'(\[Unreleased\]: .*\n)',
        f'\\1{new_link_def}\n',
        bottom_section
    )

def update_changelog():
    new_version = os.environ.get("NEW_VERSION")
    pr_number = os.environ.get("PR_NUMBER")

    if new_version and not validate_version(new_version):
        print(f"Error: Invalid version format '{new_version}'. Valid formats: 'x.y.z-a.b.c' or 'x.y.z-a.b.c-rc.n'")
        exit(1)

    if not new_version or not pr_number:
        print("Error: NEW_VERSION and PR_NUMBER environment variables must be set")
        exit(1)

    current_date, changelog_path = get_date_and_path()
    content = changelog_path.read_text()

    version_pattern = re.compile(r'## \[' + re.escape(new_version) + r'\]', re.MULTILINE)
    if version_pattern.search(content):
        print(f"Version {new_version} already exists in {changelog_path}, skipping update")
        return

    upper_section, bottom_section = split_changelog_content(content)
    unreleased_match = find_unreleased_section(upper_section)

    prev_version, unreleased_section_end, unreleased_content = find_previous_version(upper_section, unreleased_match)

    nearcore_version = extract_nearcore_version(new_version)
    new_entry = f"* chore: bump nearcore to {nearcore_version} in [#{pr_number}]"

    pr_link = f"[#{pr_number}]: {REPO_URL}/pull/{pr_number}"

    modified_unreleased_content = create_new_entry(unreleased_content, new_entry)
    modified_unreleased_content = add_pr_link(modified_unreleased_content, pr_link)

    new_version_section = f"## [{new_version}] {current_date}\n\n{modified_unreleased_content}"

    empty_unreleased = f"## [Unreleased]\n\n"
    updated_upper_section = (
        upper_section[:unreleased_match.end()] +
        "\n\n" +
        new_version_section +
        "\n\n" +
        upper_section[unreleased_section_end:]
    )

    updated_upper_section = re.sub(
        r'## \[Unreleased\].*?(?=## \[|$)',
        empty_unreleased,
        updated_upper_section,
        count=1,
        flags=re.DOTALL
    )

    updated_bottom_section = update_link_definitions(bottom_section, new_version, prev_version)
    updated_content = updated_upper_section + updated_bottom_section

    changelog_path.write_text(updated_content)
    print(f"Successfully updated {changelog_path} with new version {new_version}")

if __name__ == "__main__":
    update_changelog()

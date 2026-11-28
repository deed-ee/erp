"""Validate portable erp4us skill metadata and local guidance links."""

from pathlib import Path
import re
import sys
from urllib.parse import unquote, urlsplit

import yaml


SKILLS = {
    "erp4us-angular", "erp4us-code-review", "erp4us-contracts", "erp4us-engineering",
    "erp4us-infrastructure", "erp4us-messaging", "erp4us-persistence", "erp4us-refactoring",
    "erp4us-rust", "erp4us-security", "erp4us-service", "erp4us-testing",
}
NAME = re.compile(r"[a-z0-9]+(?:-[a-z0-9]+)*")
# Project guidance uses inline links, with no spaces or nested parentheses in paths.
LINK = re.compile(r"\]\(([^)\s]+)\)")
TOOL_ROOTS = (".claude/skills", ".copilot/skills", ".github/skills", ".opencode/skills")
AGENT_DOCS = (
    "AGENTS.md", "CLAUDE.md", "services/AGENTS.md", "ui/AGENTS.md",
    "infrastructure/AGENTS.md", "contracts/AGENTS.md", ".github/copilot-instructions.md",
    "CONTRIBUTING.md", ".github/PULL_REQUEST_TEMPLATE.md",
)


def frontmatter(text):
    lines = text.splitlines()
    if not lines or lines[0] != "---" or "---" not in lines[1:]:
        raise ValueError("expected YAML frontmatter delimited by ---")
    end = lines.index("---", 1)
    source = "\n".join(lines[1:end])
    ast = yaml.compose(source, Loader=yaml.SafeLoader)
    if not isinstance(ast, yaml.MappingNode):
        raise ValueError("frontmatter must be a YAML mapping")
    keys = [key.value for key, _ in ast.value if key.tag == "tag:yaml.org,2002:str"]
    if len(keys) != len(ast.value) or len(keys) != len(set(keys)):
        raise ValueError("frontmatter keys must be unique strings")
    return yaml.safe_load(source), "\n".join(lines[end + 1:])


def check_skill(path, text):
    try:
        data, body = frontmatter(text)
    except (ValueError, yaml.YAMLError) as exc:
        return [f"{path}: {exc}"], text
    errors = []
    name = data.get("name")
    if not isinstance(name, str) or len(name) > 64 or not NAME.fullmatch(name) or name != path.parent.name:
        errors.append(f"{path}: name must be lowercase kebab-case and match its directory")
    description = data.get("description")
    if not isinstance(description, str) or not description.strip() or len(description) > 1024:
        errors.append(f"{path}: description must be a non-empty string of at most 1024 characters")
    if set(data) - {"name", "description", "license", "compatibility", "metadata"}:
        errors.append(f"{path}: unsupported frontmatter; skills cannot grant tools or add execution hooks")
    for field, limit in (("license", None), ("compatibility", 500)):
        if field in data and (
            not isinstance(data[field], str) or not data[field].strip()
            or (limit is not None and len(data[field]) > limit)
        ):
            errors.append(f"{path}: invalid {field}")
    if "metadata" in data and (
        not isinstance(data["metadata"], dict)
        or any(not isinstance(key, str) or not isinstance(value, str) for key, value in data["metadata"].items())
    ):
        errors.append(f"{path}: metadata must map strings to strings")
    if not body.strip():
        errors.append(f"{path}: missing skill instructions")
    return errors, body


def check_links(root, path, text):
    targets, errors = set(), []
    for raw in LINK.findall(text):
        url = urlsplit(raw.strip("<>"))
        if url.scheme in {"http", "https", "mailto"}:
            continue
        relative = unquote(url.path)
        if not relative and not url.scheme and not url.netloc:
            continue
        target = (path.parent / relative).resolve()
        if (url.scheme or url.netloc or Path(relative).is_absolute()
                or not target.is_relative_to(root) or not target.exists()):
            errors.append(f"{path.relative_to(root)}: unresolved repository link {raw}")
        else:
            targets.add(target)
    return errors, targets


def check(root):
    errors = []
    canonical = root / ".agents/skills"
    standards = root / "docs/engineering"
    expected = {canonical / name / "SKILL.md" for name in SKILLS}
    found = set(canonical.rglob("SKILL.md"))
    for path in sorted(canonical.glob("*.md")):
        errors.append(f"{path.relative_to(root)}: place skills in named directories, not flat files")
    for path in sorted(expected - found):
        errors.append(f"{path.relative_to(root)}: missing required skill")
    for path in sorted(found - expected):
        errors.append(f"{path.relative_to(root)}: unexpected skill; register it in the validator and workflow")
    for path in sorted(found & expected):
        if path.is_symlink() or path.parent.is_symlink():
            errors.append(f"{path.relative_to(root)}: canonical skill must not be a symlink")
            continue
        issues, body = check_skill(path.relative_to(root), path.read_text(encoding="utf-8"))
        link_issues, targets = check_links(root, path, body)
        errors.extend(issues + link_issues)
        if not any(target.is_relative_to(standards) and target.suffix == ".md" for target in targets):
            errors.append(f"{path.relative_to(root)}: link to a canonical engineering standard")
    for directory in TOOL_ROOTS:
        if (root / directory).exists() or (root / directory).is_symlink():
            errors.append(f"{directory}: competing skill root; use .agents/skills and instruction pointers")
    guidance = [root / path for path in AGENT_DOCS]
    guidance.extend(standards.glob("*.md"))
    guidance.extend((root / ".github/instructions").glob("*.md"))
    for path in guidance:
        if not path.is_file():
            errors.append(f"{path.relative_to(root)}: missing guidance file")
            continue
        link_issues, targets = check_links(root, path, path.read_text(encoding="utf-8"))
        errors.extend(link_issues)
        if path == standards / "agent-workflow.md":
            for skill in sorted(expected - targets):
                errors.append(f"{path.relative_to(root)}: missing workflow pointer to {skill.parent.name}")
    return errors


if __name__ == "__main__":
    errors = check(Path(__file__).resolve().parents[2])
    if errors:
        print("\n".join(errors), file=sys.stderr)
        sys.exit(1)
    print(f"Validated {len(SKILLS)} skills and local guidance links")

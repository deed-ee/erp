"""Offline catalog validation and deterministic public export. Run from any directory."""
import argparse
from datetime import date
from graphlib import CycleError, TopologicalSorter
import json
from pathlib import Path
import re
from urllib.parse import unquote

from jsonschema import Draft202012Validator, FormatChecker
import yaml

ROOT = Path(__file__).resolve().parents[2]
DOC = ROOT / 'docs'
REQ = DOC / 'requirements'


class UniqueLoader(yaml.SafeLoader):
    def construct_mapping(self, node, deep=False):
        pairs = self.construct_pairs(node, deep=deep)
        result = dict(pairs)
        if len(result) != len(pairs):
            raise ValueError(f'duplicate YAML key at {node.start_mark}')
        return result


def load(path):
    return yaml.load(path.read_text(encoding='utf-8'), Loader=UniqueLoader)


def require(condition, message):
    if not condition:
        raise ValueError(message)


def local(path, base=ROOT):
    target = (base / unquote(path)).resolve()
    require(target.is_relative_to(ROOT) and target.exists(), f'invalid internal path: {path}')
    return target


def validate(records, domains, phases, scenarios, processes):
    schema = json.loads((REQ / 'schema.json').read_text())
    Draft202012Validator.check_schema(schema)
    validator = Draft202012Validator(schema, format_checker=FormatChecker())
    ids = set()
    for r in records:
        validator.validate(r)
        key, domain = r['id'], r['domain']
        require(key not in ids, f'duplicate requirement {key}')
        ids.add(key)
        require(domain in domains and domain == key.split('-')[0], f'{key}: domain')
        require(r['capability'] == domains[domain]['capability'], f'{key}: unknown capability')
        require(r['milestone'] in phases, f'{key}: unknown milestone')
        require(set(r['scenarios']) <= scenarios.keys(), f'{key}: unknown scenario')
        for field in ('tests', 'contracts', 'decisions'):
            for p in r[field]:
                require(local(p).is_file(), f'{key}: expected file: {p}')
    for r in records:
        require(set(r['dependencies']) <= ids, f'{r["id"]}: missing dependency')
        if 'supersededBy' in r:
            require(r['supersededBy'] in ids - {r['id']}, f'{r["id"]}: invalid successor')
    try:
        tuple(TopologicalSorter({r['id']: r['dependencies'] for r in records}).static_order())
    except CycleError as exc:
        raise ValueError(f'requirement dependency cycle: {exc.args[1]}') from exc
    for k, d in domains.items():
        require(set(d['dependsOn']) <= domains.keys(), f'{k}: unknown overlapping domain')
    for k, s in scenarios.items():
        require(set(s['domains']) <= domains.keys(), f'{k}: unknown scenario domain')
        require(all(s.get(field) for field in ('company', 'actors', 'given', 'when', 'then')), f'{k}: incomplete scenario')
    require(len({p['id'] for p in processes}) == len(processes), 'duplicate process')
    for p in processes:
        require(set(p['requirements']) <= ids, f'{p["id"]}: unknown requirement')
        require(all(p.get(field) for field in ('trigger', 'actors', 'preconditions', 'flow', 'alternative',
                'exception', 'objects', 'approval', 'outcomeEvent', 'result')), f'{p["id"]}: incomplete process card')


def check_link(doc, link):
    if '://' in link:
        return
    path, _, anchor = link.partition('#')
    target = local(path, doc.parent) if path else doc
    if anchor and target.suffix == '.md':
        text = target.read_text()
        headings = re.findall(r'^#{1,6}\s+(.+?)\s*$', text, re.M)
        slugs = {re.sub(r'[^\w -]', '', h.lower()).replace(' ', '-') for h in headings}
        require(unquote(anchor) in slugs or f'id="{anchor}"' in text, f'{doc.name}: unknown fragment {link}')


def catalog():
    records = []
    for path in sorted((REQ / 'catalog').glob('*.yaml')):
        rows = load(path)
        require(rows and all(r['domain'] == path.stem for r in rows), f'wrong domain file: {path}')
        records.extend(rows)
    return dict(records=records, domains=load(DOC / 'capabilities/map.yaml'), phases=load(REQ / 'phases.yaml'),
                scenarios=load(DOC / 'scenarios/companies.yaml'), processes=load(DOC / 'processes/catalog.yaml'))


def publication(data):
    """Small static projection for consumers that do not need the full YAML records."""
    return {'schemaVersion': 1, 'requirements': [
        {key: r[key] for key in ('id', 'title', 'domain', 'capability', 'type', 'status', 'priority',
                                 'milestone', 'revision', 'scenarios', 'industries', 'dependencies',
                                 'issues', 'prs', 'tests', 'evidence')}
        for r in sorted(data['records'], key=lambda r: r['id'])
    ]}


def index(data):
    lines = ['# Business requirements index', '',
             'ERP by everyone, for everyone. Start with the [review guide](guide.md), '
             '[scenario cards](../scenarios/companies.yaml) or [backlog](backlog.md). '
             'This index is generated from YAML. All delivery status claims are bounded by the individual acceptance case.',
             '', '| ID | Requirement | Phase | Priority | Status |', '|---|---|---|---|---|']
    for r in sorted(data['records'], key=lambda r: r['id']):
        lines.append(f'| [{r["id"]}](catalog/{r["domain"]}.yaml) | {r["title"]} | '
                     f'{r["milestone"]} | {r["priority"]} | {r["status"]} |')
    return '\n'.join(lines) + '\n'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--export', type=Path)
    parser.add_argument('--audit', action='store_true')
    parser.add_argument('--refresh-index', action='store_true', help='Update generated Markdown and JSON in Git')
    args = parser.parse_args()
    data = catalog()
    validate(**data)
    roles = load(REQ / 'issue-roles.yaml')
    known_issues = {n for numbers in roles.values() for n in numbers}
    require(all(set(r['issues']) <= known_issues for r in data['records']), 'unclassified issue reference')
    for path, content in [(REQ / 'index.md', index(data)), (REQ / 'public-v1.json',
                           json.dumps(publication(data), ensure_ascii=False, indent=2) + '\n')]:
        if args.refresh_index:
            path.write_text(content, encoding='utf-8')
        else:
            require(path.is_file() and path.read_text() == content, f'Stale {path.name}; run --refresh-index')
    for folder in ('requirements', 'scenarios', 'processes', 'capabilities', 'glossary'):
        for doc in (DOC / folder).rglob('*.md'):
            for link in re.findall(r'\]\(([^)]+)\)', doc.read_text()):
                check_link(doc, link)
    if args.export:
        args.export.write_text(json.dumps({'schemaVersion': 1, **data}, ensure_ascii=False, indent=2) + '\n')
    if args.audit:
        for r in data['records']:
            if not set(r['issues']).intersection(roles['delivery']):
                print(r['id'], 'no delivery issue; discovery-only is valid')
            if r['status'] == 'implemented' and not r['tests']:
                print(r['id'], 'implementation lacks linked acceptance tests')
            if r['status'] == 'superseded' or ('reviewedOn' in r and
                    (date.today() - date.fromisoformat(r['reviewedOn'])).days > 180):
                print(r['id'], 'review age or successor link')
    print('Validated', ', '.join(f'{len(v)} {k}' for k, v in data.items()))


if __name__ == '__main__':
    main()

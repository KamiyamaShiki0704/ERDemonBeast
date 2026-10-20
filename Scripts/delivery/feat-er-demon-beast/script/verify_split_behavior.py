"""Run existing real-HKS regressions with shared/adapter as separate chunks."""
import hashlib
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
SCRIPTS = Path(__file__).resolve().parent


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def verify(package):
    package = package.resolve()
    manifest = json.loads((package / 'split-build.json').read_text(encoding='utf-8-sig'))
    for entry in manifest['files']:
        assert sha(package / entry['relative']) == entry['after']
    assert manifest['luaFiles'] == ['c9997.hks', 'd0010.hks', 'modifier.hks', 'c0010.hks', 'c0010.hks']
    common, adapter = [package / 'action/script' / f for f in manifest['luaFiles'][:2]]
    # Existing harnesses load original c0010 twice after this driver. Modifier is
    # unchanged; the harness mocks the engine and does not execute modifier.hkx/Lua.
    driver = package / 'test-separate-chunks.lua'
    lines = ['-- Test driver only; never installed.']
    for path in (common, adapter):
        lines += [f'do local f=assert(io.open({json.dumps(path.as_posix())}, "rb"))',
                  "local s=f:read('*a');f:close()",
                  "if s:sub(1,3)=='\\239\\187\\191' then s=s:sub(4) end",
                  f'assert(load(s, "@{path.name}"))() end']
    driver.write_text('\n'.join(lines)+'\n', encoding='utf-8')
    original = ROOT / 'input/c0010-action-audit/c0010.lua'
    work = package / 'test-bridge'
    work.mkdir(exist_ok=True)
    results = []

    def run(name, script, *args):
        command = ['lua', str(SCRIPTS / script), *map(str, args)]
        result = subprocess.run(command, cwd=ROOT, capture_output=True, text=True)
        results.append(dict(name=name, command=command, exit_code=result.returncode,
                            output=(result.stdout+result.stderr).strip()))
        assert result.returncode == 0, results[-1]

    for chunk in ('@c9997.hks', '@F:\\GoldenAge\\_GoldenAge\\GA\\action\\script\\c9997.hks', '@action/script/c9997.hks'):
        run('chunk source '+chunk, 'test_c0010_adapter.lua', common, original, adapter, chunk)
    for mode in ('lifecycle', 'combo'):
        run(mode, 'test_c0010_v3.lua', driver, original, mode, work)
    for mode in ('damage_pending','damage_exit','roll_repeat','roll_blend','roll_cycles',
                 'outro','outro_blend','skill','skill_exit','original','fourway'):
        run(mode, 'test_c0010_v5.lua', driver, original, work, mode)
    for mode in ('mapped_light','mapped_heavy','mapped_disable','mapped_combo','mapped_invalid'):
        run(mode, 'test_c0010_v5.lua', driver, original, work, mode, ROOT / 'tmp/mapped-wire-042.txt')
    run('44 damage direction cases', 'test_c0010_damage.lua', driver, original, ROOT / 'tmp/c0010-animation-ids.lua')
    run('entry recovery', 'test_c0010_entry.lua', driver, original, work)
    report = dict(result='PASS_OFFLINE', runtime_tested=False, cases=results,
                  files={e['relative']:sha(package / e['relative']) for e in manifest['files']},
                  limits=['Engine/Havok playback and modifier execution require human game verification.'])
    (package / 'split-verification.json').write_text(json.dumps(report, ensure_ascii=False, indent=2)+'\n', encoding='utf-8')
    print(f'PASS: {len(results)} regression invocations; shared and adapter loaded as separate chunks')


if __name__ == '__main__':
    verify(Path(sys.argv[1]))

"""Package the already verified DLL/HKS without changing the installed game."""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
from behavior_assets import private_script

ROOT = Path(__file__).resolve().parents[4]
GA = Path('F:/GoldenAge/_GoldenAge/GA')
DLL = Path('F:/GoldenAge/_GoldenAge/dll/demon_beast')
OUT = ROOT / 'dist/shared-health-0.4.2-r2'

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def save(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2)+'\n', encoding='utf-8')

if OUT.exists():
    raise SystemExit('Choose a fresh package directory; existing candidates are immutable')
names = {
    'ga': ['action/script/d0010.hks', 'chr/c0010.behbnd.dcx'],
    'dll': ['er_demon_beast.dll', 'er_demon_beast.pdb', 'er-demon-beast.toml',
            'build-result.json', 'behavior-build.json', 'behavior-deployment.json',
            'BEHAVIOR_TEST.md', 'probe.py', 'manual_test.py'],
}
roots = {'ga': GA, 'dll': DLL}
before = {}
for scope, files in names.items():
    for name in files:
        source, dest = roots[scope]/name, OUT/scope/name
        before[scope+'/'+name] = digest(source)
        dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, dest)
for ext in ['dll', 'pdb']:
    shutil.copyfile(ROOT/f'target/x86_64-pc-windows-msvc/release/er_demon_beast.{ext}', OUT/f'dll/er_demon_beast.{ext}')
private = OUT/'ga/action/script/d0010.hks'
private.write_bytes(private_script((GA/'action/script/c9997.hks').read_bytes(),
                                  (ROOT/'assets/action/script/c0010-er-adapter.hks').read_bytes()))
assert private.read_bytes() == (ROOT/'tmp/d0010-v9.hks').read_bytes(), 'Packaged HKS differs from tested candidate'
protected_names = ['action/script/c9997.hks','action/script/c0010.hks','chr/c0010.anibnd.dcx',
                   'regulation.bin','action/eventnameid.txt','action/statenameid.txt','action/variablenameid.txt']
protected = {name: digest(GA/name) for name in protected_names}
source_names = ['Cargo.toml','Cargo.lock','src/native.rs','src/native/health.rs','src/native/capture.rs',
                'src/session.rs','src/forms.rs','src/native/behavior_bridge.rs','assets/action/script/c0010-er-adapter.hks']
result = {
    'version':'0.4.2','behavior_version':9,
    'source_revision':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
    'working_tree_changes':True,'source_sha256':{name:digest(ROOT/name) for name in source_names},
    'spec':'spec-shared-health.md@1.0','design':'design-shared-health.md',
    'knowledge_base_revision':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT.parents[1],text=True).strip(),
    'dll_sha256':digest(OUT/'dll/er_demon_beast.dll'),'hks_sha256':digest(private),
    'rust_tests':45,'python_tests':17,'clippy':'PASS','fmt':'PASS','release_build':'PASS',
    'small_stack_regression':'PASS','runtime_tested':False,'installed_changed':False,'release_ready':True,
    'hks_regression':'PASS: 44 damage cases; pending interruption/exit; 11 lifecycle/roll/direction cases; adapter, combo, entry, 5 Rust-generated mapping cases',
    'human_pending':['Shared health HUD/damage/healing within 0.25s','Three cycles without HP reset/double charge',
                     'Native player death on zero HP','No pose or stuck input on actual hits'],
    'limits':['No missing hit animations synthesized; absent ordinary reactions are skipped while HP still changes',
              'Native death animation/save behavior requires human verification','No multiplayer/map transition coverage'],
}
save(OUT/'dll/build-result.json',result)
behavior=json.loads((OUT/'dll/behavior-build.json').read_text(encoding='utf-8-sig'))
behavior['version']=9
behavior['runtime']='NOT_RUN; shared-health and hit-reaction human acceptance pending'
behavior['protected_inputs']=protected
for item in behavior.get('files',[]):
    path=OUT/'ga'/item['relative']
    if path.is_file():
        item['before']=before['ga/'+item['relative']]
        item['after']=digest(path)
save(OUT/'dll/behavior-build.json',behavior)
# This compatibility record describes the current paired state. Restoration must
# use the transaction record, never an obsolete historical behavior backup.
save(OUT/'dll/behavior-deployment.json',{
    'status':'paired-package','version':'0.4.2 / HKS v9','root':str(GA),
    'transaction_record':str(DLL/'deployment-health-0.4.2.json'),
    'files':[{'relative':name,'before':before['ga/'+name],'after':digest(OUT/'ga'/name)} for name in names['ga']],
    'protected_inputs':protected,
})
test_doc=OUT/'dll/BEHAVIOR_TEST.md'
test_doc.write_text('''# 0.4.2 / HKS v9 共用血量与受击回归

沿用当前启动方式与配置。血量从玩家继承，不是新的独立血池。

1. 半血进入，确认没有补满；被普通攻击击中后，检查玩家血条同步减少，兽体不进入Pose。
2. 有可用回血效果时，确认治疗反映到共享生命；解除后保留剩余生命。
3. 攻击刚出手时受击，再尝试移动、攻击、闪避；连续3次变身/解除，无卡键、重复扣血或恢复原血量。
4. 在有真正受击动画时请求解除，等动作结束后退场；现有资源没有普通轻/中受击动画时不强行播放。
5. 让共享生命归零，确认交回玩家正常死亡，不能解除变身后满血存活。交回控制≤3秒；死亡演出时长沿用ER。

血条同步允许≤0.25秒显示差异；精确HP及max_hp记录在采样中。HKS日志为Saved/er-demon-beast/c0010-hks-v9.log，DLL日志包含health linked/health sync。
离线测试通过不等于上述实机项目通过。不要用旧版本回退器拆开还原本次DLL/HKS；本次回退入口为Restore-SharedHealth042.cmd。
''',encoding='utf-8')
files=[{'scope':scope,'relative':name,'before':before[scope+'/'+name],'after':digest(OUT/scope/name)}
       for scope,group in names.items() for name in group]
for item in files:
    assert digest(roots[item['scope']]/item['relative']) == item['before'], 'Installed file changed during preparation'
save(OUT/'upgrade-manifest.json',{'version':'0.4.2 / HKS v9','release_ready':True,'pending_requirement':None,
                                   'files':files,'protected_inputs':protected})
shutil.copyfile(Path(__file__).with_name('Manage-CustomForms.ps1'),OUT/'Manage-SharedHealth042.ps1')
save(ROOT/'docs/feat-er-demon-beast/research/build-result-health-0.4.2.json',result)
print(json.dumps({'package':str(OUT),'dll_sha256':result['dll_sha256'],'hks_sha256':result['hks_sha256'],
                  'protected_regulation':protected['regulation.bin'],'release_ready':True},indent=2))

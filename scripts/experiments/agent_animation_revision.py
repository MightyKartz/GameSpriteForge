#!/usr/bin/env python3
"""M2: immutable bad-frame candidate -> explicit one-frame replacement -> native delivery.
Synthetic defects and holds/jumps are deliberate; this is not artistic approval.
"""
import argparse
import copy
import hashlib
import html
import json
import os
from pathlib import Path
import re
import shutil
import subprocess

from PIL import Image, ImageDraw
from agent_resource_baseline import make_inputs, CHECKER


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def inventory(root):
    return {p.relative_to(root).as_posix(): digest(p) for p in root.rglob('*') if p.is_file()}


def save(path, value):
    path.write_text(json.dumps(value, indent=2, ensure_ascii=False)+'\n', encoding='utf-8')


def check(args):
    root=args.output.resolve(); root.mkdir(parents=True, exist_ok=False)
    logs=root/'logs'; logs.mkdir()
    env=dict(os.environ, FORGE_JOB_STORE=str(root/'jobs'), FORGE_PLAN_STORE=str(root/'plans'),
             FORGE_CONFIG_DIR=str(root/'config'), FORGE_GODOT_PATH=str(args.godot.absolute()),
             FORGE_REAL_PROVIDER_MAX_REQUESTS='0')
    env.pop('FORGE_REAL_PROVIDER_ACCEPT', None)
    summary={'runnerSha256':digest(Path(__file__)), 'checkerSha256':digest(CHECKER), 'ok':False, 'visualReview':'not_assessed', 'providerCalls':0, 'calls':[]}
    def cli(label, *argv, reject=False):
        p=subprocess.run([str(args.forge.absolute()), *map(str,argv), '--json'], env=env,
                         capture_output=True, text=True, encoding='utf-8', timeout=240)
        save(logs/(label+'.json'), {'stdout':p.stdout,'stderr':p.stderr,'exitCode':p.returncode})
        value=json.loads(p.stdout)
        assert value['ok'] is (not reject) and (p.returncode!=0)==reject, (label,value)
        summary['calls'].append({'label':label,'exitCode':p.returncode})
        return value['error'] if reject else value['data']
    def execute(label, recipe, kind='prepare-character'):
        path=root/'requests'/(label+'.json'); save(path,recipe)
        plan=cli(label+'-plan','plan',kind,'--request',path)
        assert plan['estimate']['maximumProviderRequests']==0
        return cli(label+'-execute','plan','execute','--token',plan['token'],'--wait')
    def pack_of(job):
        return Path(next(a['path'] for a in job['artifacts'] if a['kind']=='gsfpack'))
    def lock(recipe):
        paths=set()
        for action in recipe['animations']:
            source=action['input']
            paths.update(source['paths'] if source['kind']=='png_sequence' else [source['path']])
        recipe['sourceLocks']=[{'path':p,'sha256':digest(Path(p))} for p in sorted(paths)]
    try:
        summary['doctor']=cli('doctor','doctor'); summary['forgeSha256']=digest(args.forge.resolve())
        recipes,_=make_inputs(root)
        good=recipes['synthetic'][1]
        # Use known-order individual frames for an explicit local replacement recipe.
        for action in good['animations']:
            action['input']={'kind':'png_sequence','paths':[str(root/'inputs'/f"synthetic-{action['name']}-{i}.png") for i in range(3)]}
        hold=root/'inputs'/'intentional-hold.png'; shutil.copyfile(good['animations'][1]['input']['paths'][0],hold)
        jump=root/'inputs'/'intentional-jump.png'
        image=Image.open(good['animations'][1]['input']['paths'][2]).convert('RGBA')
        shifted=Image.new('RGBA',image.size); shifted.paste(image,(0,-6)); shifted.save(jump)
        good['animations'][1]['input']['paths'][1:]=[str(hold),str(jump)]
        bad=copy.deepcopy(good)
        clipped=root/'inputs'/'attack-1-clipped.png'
        image=Image.open(good['animations'][2]['input']['paths'][1]).convert('RGBA')
        ImageDraw.Draw(image).line((39,30,72,30),fill=(40,190,220,128),width=2); image.save(clipped)
        bad['animations'][2]['input']['paths'][1]=str(clipped)
        empty=root/'inputs'/'empty.png'; Image.new('RGBA',(64,64)).save(empty)
        original=inventory(root/'inputs')
        summary['inputSha256']=original
        library=root/'library'
        cli('library','project','init','--path',library,'--name','M2 fixture','--local-assets')
        for recipe in (good,bad):
            recipe['assetProject']={'projectPath':str(library),'assetId':'hero'}; lock(recipe)
        # M1 diagnostics already cover missing input, timing, and invalid fixed-grid regions.
        summary['rejections']={}
        for defect in ('missing','duration','region'):
            request=copy.deepcopy(bad); request.pop('sourceLocks')
            action=request['animations'][2]
            if defect=='missing': action['input']['paths'][1]=str(root/'missing.png')
            if defect=='duration': action['frameDurationsMs'][1]=0
            if defect=='region': action['input']={'kind':'sprite_sheet','path':str(clipped),'split':{'mode':'fixed_grid','frameWidth':64,'frameHeight':64,'columns':2,'rows':1}}
            path=root/'requests'/(defect+'.json'); save(path,request)
            error=cli(defect,'plan','prepare-character','--request',path,reject=True)
            assert 'attack' in error['message'], error
            summary['rejections'][defect]=error
        request=copy.deepcopy(bad);request['animations'][2]['input']['paths'][1]=str(empty)
        request['quality']['requireGameReady']=True;lock(request)
        empty_job=execute('empty',request)
        assert empty_job['lifecycle_state']=='awaiting_review',empty_job
        empty_report=cli('empty-report','job','report','--id',empty_job['job_id'])
        empty_quality=empty_report['reports']['animation_quality_report']
        issue=next(i for a in empty_quality['animations'] if a['name']=='attack' for i in a['report']['pixelDiagnostics']['issues'] if i['code']=='empty_frame')
        assert issue['frameIndex']==1 and issue['certainty']=='deterministic'
        empty_before=inventory(Path(empty_job['job_dir']))
        cli('empty-cannot-approve','job','review','--id',empty_job['job_id'],'--accept','--reason','Negative fixture: blocked must not be bypassed',reject=True)
        assert empty_before==inventory(Path(empty_job['job_dir']))
        before_job=execute('before',bad); assert before_job['lifecycle_state']=='succeeded',before_job
        before=pack_of(before_job); before_bytes=inventory(before)
        report=cli('before-report','job','report','--id',before_job['job_id'])
        quality=report['reports']['animation_quality_report']
        attack=next(a['report'] for a in quality['animations'] if a['name']=='attack')
        edge=next(i for i in attack['pixelDiagnostics']['issues'] if i['code']=='canvas_edge_contact')
        assert edge['frameIndex']==1 and edge['severity']=='warning' and edge['certainty']=='review_required'
        walk=next(a['report'] for a in quality['animations'] if a['name']=='walk')
        assert any(i['code']=='identical_visible_frames' and i['frameIndex']==1 for i in walk['pixelDiagnostics']['issues'])
        before_revision=cli('before-history','asset','history','--project',library,'--id','hero')[0]['revision']
        cli('retain-before','asset','retain','--project',library,'--id','hero','--revision',before_revision)
        evidence=root/'review.txt';evidence.write_text('Synthetic technical fixture only: attack frame 1 reaches the edge. No artwork approval.\n',encoding='utf-8')
        cli('review-before','asset','review','--project',library,'--id','hero','--revision',before_revision,'--domain','technical','--verdict','needs_review','--reviewer','fixture','--statement','Inspect attack frame 1; deliberate clipping fixture.','--evidence',evidence)
        old_reviews=cli('old-reviews','asset','reviews','--project',library,'--id','hero','--revision',before_revision)
        game=root/'game'; game.mkdir(); (game/'project.godot').write_text('config_version=5\n[application]\nconfig/name="M2"\n',encoding='utf-8')
        custom=game/'gameplay.gd';custom.write_text('extends Node\nvar attack_damage := 7\n',encoding='utf-8')
        def install(label,pack):
            job=execute(label,{'schemaVersion':'1','packPath':str(pack),'projectPath':str(game),'target':'addons/forge_assets/hero','assetKey':'hero','providerRefs':[]},'install-godot')
            assert job['lifecycle_state']=='succeeded',job
            cli(label+'-verify','godot','verify-install','--project',game,'--asset-key','hero','--pack',pack)
        install('before-install',before)
        game_contract={p.name:digest(p) for p in (custom,game/'project.godot')}
        # One existing request field changes; locks bind the explicitly chosen replacement.
        after_job=execute('after',good);assert after_job['lifecycle_state']=='succeeded',after_job
        after=pack_of(after_job);cli('after-validate','pack','validate','--path',after)
        after_report=cli('after-report','job','report','--id',after_job['job_id'])
        assert not any(i['code']=='canvas_edge_contact' for a in after_report['reports']['animation_quality_report']['animations'] for i in a['report']['pixelDiagnostics']['issues'])
        before_manifest=json.loads((before/'assets/manifest.json').read_text());after_manifest=json.loads((after/'assets/manifest.json').read_text())
        assert before_manifest['animations']==after_manifest['animations']
        assert before_manifest['anchor']==after_manifest['anchor']
        changed=[]
        for action in after_manifest['animations']:
            for index, global_index in enumerate(action['frames']):
                name=f'assets/frames/frame_{global_index+1:03}.png'
                if (before/name).read_bytes()!=(after/name).read_bytes(): changed.append([action['name'],index])
        assert changed==[['attack',1]],changed
        history=cli('after-history','asset','history','--project',library,'--id','hero')
        after_revision=next(row['revision'] for row in history if row['revision']!=before_revision)
        assert not cli('new-reviews','asset','reviews','--project',library,'--id','hero','--revision',after_revision)
        assert old_reviews==cli('old-reviews-preserved','asset','reviews','--project',library,'--id','hero','--revision',before_revision)
        library_before=inventory(library)
        preview=cli('comparison','asset','preview','--project',library,'--id','hero','--revision',before_revision,'--revision',after_revision,'--out',root/'comparison')
        assert preview['revisions']==2 and not preview['issues']
        page=(root/'comparison/index.html').read_text(encoding='utf-8')
        players=[json.loads(html.unescape(v)) for v in re.findall(r'data-forge-animation="([^"]+)"',page)]
        assert len(players)==2 and 'data-sync' in page and 'data-guides' in page
        for player,pack in zip(players,(before,after)):
            assert player['anchor']==[32,52]
            for animation in player['animations']:
                source=next(a for a in after_manifest['animations'] if a['name']==animation['name'])
                assert animation['durationsMs']==source['frameDurationsMs']
            for i,url in enumerate(player['urls']): assert (root/'comparison'/url).read_bytes()==(pack/f'assets/frames/frame_{i+1:03}.png').read_bytes()
        assert library_before==inventory(library)
        install('after-install',after)
        contract={'characters':[{'id':'hero','scene':'res://addons/forge_assets/hero/forge_animated_sprite.tscn','size':[64,64],'anchor':[32,52],
          'actions':[{'name':a['name'],'frames':a['input']['paths'],'durationsMs':a['frameDurationsMs'],'loop':a['loop']} for a in good['animations']]}], 'static':[], 'audio':[]}
        save(game/'acceptance.json',contract);shutil.copyfile(CHECKER,game/'verify.gd')
        p=subprocess.run([str(args.godot.absolute()),'--headless','--path',str(game),'--script','res://verify.gd'],env=env,capture_output=True,text=True,encoding='utf-8',timeout=120)
        save(logs/'native.json',{'stdout':p.stdout,'stderr':p.stderr,'exitCode':p.returncode})
        assert p.returncode==0 and 'RESOURCE_TASK_PASS:1' in p.stdout.splitlines() and 'SCRIPT ERROR' not in p.stderr+p.stdout,(p.stdout,p.stderr)
        assert game_contract=={p.name:digest(p) for p in (custom,game/'project.godot')}
        assert original==inventory(root/'inputs') and before_bytes==inventory(before)
        assert report['providerRequestCount']==after_report['providerRequestCount']==0
        summary.update(ok=True,changedFrames=changed,sourceAndOldPackUnchanged=True,oldReviewsPreserved=True,newRevisionReviews=[],gameConfigurationUnchanged=True,
                       beforeRevision=before_revision,afterRevision=after_revision,emptyFrameIssue=issue,edgeIssue=edge,nativeAcceptance=True,
                       checks=['missing_frame','invalid_region','zero_duration','empty_frame_blocked','clipping_review_hint','intentional_hold_and_jump_preserved','single_frame_only','preview_native_timing','fresh_review','native_delivery'])
    except Exception as error:
        summary['error']=str(error)
        raise
    finally:
        save(root/'summary.json',summary)
    return summary

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--forge',required=True,type=Path);parser.add_argument('--godot',required=True,type=Path);parser.add_argument('--output',required=True,type=Path)
    result=check(parser.parse_args()); print(json.dumps({'ok':result['ok'],'checks':result['checks']}))

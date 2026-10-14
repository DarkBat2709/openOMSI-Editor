// The map builder has its own entry, independent of the selected driving duty.
import { html, useState, useEffect } from '../../vendor/preact-htm.js';
import { useStore, set, toast } from '../store.js';
import { t } from '../i18n.js';
import { call } from '../api.js';
import { Button, Card, Field, Input, PageHead, Select } from '../ui.js';

export function EditorPage() {
  const s = useStore();
  const [name, setName] = useState('');
  const [author, setAuthor] = useState('');
  const [description, setDescription] = useState('');
  const [chosen, setChosen] = useState('');
  const [busy, setBusy] = useState(false);
  const maps = s.maps || [];
  const map = chosen || maps[0]?.file || '';
  const running = (s.poll.instances || []).some(i => i.running);
  const act = async (create) => {
    if (busy || running) return;
    setBusy(true);
    try {
      let file = map;
      if (create) {
        file = (await call('editor_create', { name, author, description })).map;
        setChosen(file);
        set({ maps: await call('maps') });
      }
      await call('editor_open', { map: file });
    } catch (e) { toast(e.message, 'error'); }
    finally { setBusy(false); }
  };
  return html`<${PageHead} title="Editor" text=${running
    ? 'Please end the current session before starting a map in the editor.'
    : 'Create a new map or edit an existing map directly.'} />
    <${Card} title="New empty map">
      <${Field} label="Map name"><${Input} value=${name} onInput=${setName} /></${Field}>
      <${Field} label="Author (optional)"><${Input} value=${author} onInput=${setAuthor} /></${Field}>
      <${Field} label="Description"><${Input} value=${description} onInput=${setDescription} /></${Field}>
      <p class="muted">${t('Start with one flat tile (300 × 300 m), a ground texture and a free camera. Add more tiles in the editor.')}</p>
      <${Button} kind="primary" label="Create and edit map" disabled=${busy || running || !name.trim()} onClick=${() => act(true)} />
    </${Card}>
    <${Card} title="Edit existing map">
      <${Select} value=${map} options=${maps.map(m => [m.file, m.friendly || m.name])} raw onChange=${setChosen} disabled=${busy || running} />
      ${!maps.length && html`<p>${t('No maps found. Please check Setup.')}</p>`}
      <${Button} kind="primary" label="Open in editor" disabled=${busy || running || !map} onClick=${() => act(false)} />
    </${Card}>`;
}

export function EditorUpdates() {
  const [channels, setChannels] = useState([]);
  useEffect(() => {
    let active = true, timer;
    const refresh = async () => {
      try {
        const next = await call('editor_updates');
        if (!active) return;
        setChannels(next);
        if (next.some(c => c.state === 'checking')) timer = setTimeout(refresh, 800);
      } catch (e) { if (active) toast(e.message, 'error'); }
    };
    refresh();
    return () => { active = false; clearTimeout(timer); };
  }, [channels.some(c => c.state === 'checking')]);
  const check = async i => {
    try { setChannels(await call('editor_updates', { check: i })); }
    catch (e) { toast(e.message, 'error'); }
  };
  return html`<div class="setting-row full"><div>
    ${channels.map((c, i) => html`<div class="field">
      <b>${t(i === 0 ? 'openOMSI · separate game installation' : 'openOMSI Editor · editor package')}</b>
      <div class="row wrap"><${Button} small label="Check now" disabled=${c.state === 'checking'} onClick=${() => check(i)} />
        <span>${c.current} ${c.state === 'up_to_date' ? t('Up to date') : c.message || t('Version and installation are checked separately')}</span>
        ${c.release && html`<${Button} small label="Download" onClick=${() => call('open_url', { url: c.release }).catch(e => toast(e.message, 'error'))} />`}
      </div><small class="muted">${c.path || ''}</small>
    </div>`)}
    <p class="muted">${t('Updates are installed manually. The editor keeps its bundled engine; updating the separate game does not upgrade the editor. Back up maps before saving with a newer version.')}</p>
  </div></div>`;
}

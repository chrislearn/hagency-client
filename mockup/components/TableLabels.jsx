'use client';

import { useEffect } from 'react';

/* Phones show each table row as a card, and a card cell needs its column's
 * name beside the value. Rather than thread a label through every table, this
 * copies each header's text onto the cells below it (data-label), and keeps
 * doing so as tables re-render. It only sets an attribute React never owns. */
const TONES = Object.fromEntries([
  ...['active', 'running', 'online', 'ready', 'approved', 'done', 'complete', 'registered', 'verified', 'accepted', '运行中', '在线', '活跃'].map((w) => [w, 'ok']),
  ...['pending', 'reserved', 'queued', 'waiting', 'in progress', 'in_progress', 'stopping', '待定', '等待中'].map((w) => [w, 'warn']),
  ...['failed', 'rejected', 'revoked', 'blocked', 'error', '失败', '已拒绝', '已撤销'].map((w) => [w, 'bad']),
  ...['offline', 'stopped', 'ended', 'retired', '离线', '已停止'].map((w) => [w, 'off']),
  ...['unknown', 'not_applied', 'not applied', '未知'].map((w) => [w, 'none']),
]);

export default function TableLabels() {
  useEffect(() => {
    const main = document.querySelector('main');
    if (!main) return undefined;
    const label = () => {
      for (const table of main.querySelectorAll('table')) {
        const head = table.querySelector('thead tr') ?? table.querySelector('tr');
        if (!head || !head.querySelector('th')) continue;
        const names = [...head.children].map((cell) => cell.textContent.trim());
        for (const row of table.querySelectorAll('tr')) {
          if (row === head) continue;
          [...row.children].forEach((cell, index) => {
            if (cell.tagName === 'TD' && names[index] && cell.getAttribute('data-label') !== names[index]) cell.setAttribute('data-label', names[index]);
            // A cell that holds only a state word gets that word's tone, so a
            // status reads as a dot plus its word rather than plain text.
            const word = cell.tagName === 'TD' && cell.children.length === 0 ? cell.textContent.trim().toLowerCase() : '';
            const tone = TONES[word] ?? null;
            if (tone && cell.getAttribute('data-tone') !== tone) cell.setAttribute('data-tone', tone);
            else if (!tone && cell.hasAttribute('data-tone')) cell.removeAttribute('data-tone');
            // One long unbroken token (an id, a timestamp) may wrap anywhere;
            // ordinary words never break mid-word.
            const text = cell.tagName === 'TD' && cell.children.length === 0 ? cell.textContent.trim() : '';
            const time = /^\d{4}-\d{2}-\d{2}T[\d:.]+Z$/.test(text);
            const token = !time && /^\S{22,}$/.test(text);
            if (time !== cell.hasAttribute('data-time')) cell.toggleAttribute('data-time', time);
            if (token !== cell.hasAttribute('data-token')) cell.toggleAttribute('data-token', token);
            // A truncated id keeps its full value one hover away.
            if (token && cell.title !== text) cell.title = text;
          });
        }
      }
    };
    const tiles = () => {
      // A row of count tiles that are all zero reads as one quiet summary line.
      for (const group of main.querySelectorAll('.cards')) {
        const values = [...group.querySelectorAll('.card .val')].map((v) => v.textContent.trim());
        const zero = values.length > 1 && values.every((v) => v === '0');
        if (zero !== group.classList.contains('all-zero')) group.classList.toggle('all-zero', zero);
      }
    };
    const pass = () => { label(); tiles(); };
    pass();
    const observer = new MutationObserver(pass);
    observer.observe(main, { childList: true, subtree: true, characterData: true });
    return () => observer.disconnect();
  }, []);
  return null;
}

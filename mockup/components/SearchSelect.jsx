'use client';

import { useMemo, useState } from 'react';
import { useT } from '@/components/Prefs';

/*
 * A select with a search box above it.
 *
 * The pickers used to be bare <select> elements over one cursor page of 16
 * rows. Nothing could reach a row past the first page: there was no search, no
 * filter, and no name for a selection the page did not contain — the reader got
 * "Selected engagement outside this page" exactly when they had just come from
 * that row. Typing narrows the list here; the remembered name is supplied by the
 * caller, which is the only place that knows what to call an id it cannot see.
 *
 * It stays a real <select> (not a div-combobox) so keyboard selection,
 * middle-click and the existing browser tests' selectOption() all keep working.
 */
export default function SearchSelect({ id, value, onChange, options, outside, empty, placeholder }) {
  const t = useT();
  const [query, setQuery] = useState('');
  const shown = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return options;
    return options.filter((o) => o.label.toLowerCase().includes(q));
  }, [options, query]);

  if (!options.length && !value) return <p>{empty}</p>;

  return (
    <>
      <input
        type="search"
        className="search"
        id={`${id}-search`}
        aria-label={t('common.search')}
        placeholder={placeholder ?? t('common.searchPlaceholder')}
        value={query}
        onChange={(event) => setQuery(event.target.value)}
      />
      <select id={id} value={value ?? ''} onChange={onChange}>
        {/* The selection itself is always offered, named where the caller could
            recover a name and honestly branded where it could not — never a
            blank option that silently changes what the page is showing. */}
        {value && !options.some((o) => o.value === value) && <option value={value}>{outside}</option>}
        {shown.map((o) => <option key={o.value} value={o.value}>{o.label}</option>)}
      </select>
    </>
  );
}

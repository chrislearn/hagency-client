import NativeHome from '@/components/NativeHome';

/*
 * The NATIVE console's root document only. app/page.jsx stays untouched —
 * the retained console's root IS 我的资源 — and build-native-console.mjs
 * stages this file as the staged tree's app/page.jsx, where Next renders it
 * as the exported index.html the server maps to /console/.
 */
export default function Page() {
  return <NativeHome />;
}

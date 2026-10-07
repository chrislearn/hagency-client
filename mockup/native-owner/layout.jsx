import { Roboto, Noto_Sans_SC } from 'next/font/google';
import './globals.css';
import OwnerShell from '@/components/OwnerShell';
import { PrefsProvider } from '@/components/Prefs';

const roboto = Roboto({ subsets: ['latin'], weight: ['400', '500'], display: 'swap', variable: '--font-roboto' });
const notoSansSC = Noto_Sans_SC({ subsets: ['latin'], weight: ['400', '500'], display: 'swap', variable: '--font-opensans-sc' });
export const metadata = { title: 'Hagency Client', description: 'Manage your Matrix agents and local Codex resources.' };

export default function OwnerLayout({ children }) {
  return <html lang="en" className={`${roboto.variable} ${notoSansSC.variable}`} suppressHydrationWarning>
    <head><script dangerouslySetInnerHTML={{ __html: `(function(){try{var t=localStorage.getItem('hagency.theme');if(t&&t!=='system')document.documentElement.setAttribute('data-theme',t);if(localStorage.getItem('hagency.locale')==='zh')document.documentElement.setAttribute('lang','zh-CN');}catch(e){}})();` }} /></head>
    <body><PrefsProvider><OwnerShell>{children}</OwnerShell></PrefsProvider></body>
  </html>;
}

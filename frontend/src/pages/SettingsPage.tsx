import SettingsSharing from '@/components/SettingsSharing';
import { useTranslation } from 'react-i18next';

import UserSettingsBlock from '@/components/user-settings-block';
import TelegramSettings from '@/components/telegram-settings';

export function SettingsPage() {
  const { t } = useTranslation();

  // Reset to defaults functionality has been removed.

  return (
    <div className="space-y-8">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold">{t('settings.title')}</h1>
          <p className="text-muted-foreground">{t('settings.subtitle')}</p>
        </div>
      </div>

      <UserSettingsBlock />

      <TelegramSettings />

      <SettingsSharing />
    </div>
  );
}

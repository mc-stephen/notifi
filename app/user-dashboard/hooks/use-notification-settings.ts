import { useCallback, useState } from "react";
import { NOTIFICATION_SETTINGS } from "@/lib/constants";

export function useNotificationSettings() {
  const [settings, setSettings] = useState(() =>
    NOTIFICATION_SETTINGS.map((cat) => ({
      ...cat,
      settings: cat.settings.map((s) => ({ ...s })),
    })),
  );

  const toggle = useCallback(
    (categoryId: string, key: string, channel: "email" | "inApp") => {
      setSettings((prev) =>
        prev.map((cat) =>
          cat.id !== categoryId
            ? cat
            : {
                ...cat,
                settings: cat.settings.map((s) =>
                  s.key !== key
                    ? s
                    : {
                        ...s,
                        [channel]: !s[channel],
                      },
                ),
              },
        ),
      );
    },
    [],
  );

  const setAll = useCallback((categoryId: string, enabled: boolean) => {
    setSettings((prev) =>
      prev.map((cat) =>
        cat.id === categoryId
          ? {
              ...cat,
              settings: cat.settings.map((s) => ({
                ...s,
                email: enabled,
                inApp: enabled,
              })),
            }
          : cat,
      ),
    );
  }, []);

  return { categories: settings, toggle, setAll };
}

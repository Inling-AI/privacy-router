import 'package:shared_preferences/shared_preferences.dart';

import 'strings.g.dart';

abstract final class ConsoleLanguage {
  static const _key = 'console.locale';
  static Future<void> restore() async {
    final stored = await SharedPreferencesAsync().getString(_key);
    final locale = AppLocale.values
        .where((value) => value.languageTag == stored)
        .firstOrNull;
    if (locale == null) {
      await LocaleSettings.useDeviceLocale();
    } else {
      await LocaleSettings.setLocale(locale);
    }
  }

  static Future<void> select(AppLocale locale) async {
    await LocaleSettings.setLocale(locale);
    await SharedPreferencesAsync().setString(_key, locale.languageTag);
  }

  static String label(AppLocale locale) => switch (locale) {
    AppLocale.en => 'English',
    AppLocale.zhCn => '简体中文',
  };
}

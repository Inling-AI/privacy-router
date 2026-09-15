import 'package:cupertino_ui/cupertino_ui.dart';
import 'package:liquid_glass_widgets/liquid_glass_widgets.dart';

import '../i18n/language.dart';
import '../i18n/strings.g.dart';

class LanguageMenu extends StatelessWidget {
  const LanguageMenu({super.key});
  @override
  Widget build(BuildContext context) => GlassPullDownButton(
    icon: const Icon(CupertinoIcons.globe),
    semanticLabel: context.t.language,
    items: items(context),
  );

  static List<GlassMenuItem> items(BuildContext context) => [
    for (final locale in AppLocale.values)
      GlassMenuItem(
        title: ConsoleLanguage.label(locale),
        icon: const Icon(CupertinoIcons.globe, size: 18),
        isSelected: TranslationProvider.of(context).locale == locale,
        onTap: () => ConsoleLanguage.select(locale),
      ),
  ];
}

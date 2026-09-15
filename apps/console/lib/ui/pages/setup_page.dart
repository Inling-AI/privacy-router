/// 首次安装的自助初始化页。
///
/// 服务端报告「还没有管理员」时控制台直接进入这里：填账号、设口令、创建。创建成功即已
/// 登录，因此整个首次安装过程不需要任何命令行操作。
library;

import 'package:cupertino_ui/cupertino_ui.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:liquid_glass_widgets/liquid_glass_widgets.dart';

import '../../i18n/strings.g.dart';
import '../../state/providers.dart';
import '../../theme/elevation.dart';
import '../../theme/palette.dart';
import '../../theme/tokens.dart';
import '../language_menu.dart';
import '../shell.dart' show ConsoleCanvas;
import '../widgets.dart';

class SetupPage extends ConsumerStatefulWidget {
  const SetupPage({super.key});

  @override
  ConsumerState<SetupPage> createState() => _SetupPageState();
}

class _SetupPageState extends ConsumerState<SetupPage> {
  final _username = TextEditingController(text: 'admin');
  final _password = TextEditingController();
  final _confirmation = TextEditingController();
  bool _obscure = true;

  /// 表单自己能判定的错误。服务端拒绝的消息走会话状态，两者不重复表达。
  String? _localError;

  @override
  void dispose() {
    _username.dispose();
    _password.dispose();
    _confirmation.dispose();
    super.dispose();
  }

  void _submit() {
    final username = _username.text.trim();
    final password = _password.text;
    String? localError;
    if (username.isEmpty) {
      localError = context.t.setup.usernameRequired;
    } else if (password.isEmpty) {
      localError = context.t.setup.passwordRequired;
    } else if (password != _confirmation.text) {
      // 确认字段只存在于表单：服务端根本看不到它，因此只能在这里比对。
      localError = context.t.setup.passwordMismatch;
    }
    setState(() => _localError = localError);
    if (localError == null) {
      ref.read(sessionProvider.notifier).setup(username, password);
    }
  }

  @override
  Widget build(BuildContext context) {
    final palette = ConsolePalette.of(context);
    final session = ref.watch(sessionProvider);
    final busy = session is InFlight;
    final failure = _localError ?? (session is AuthenticationFailed ? session.message : null);

    return GlassScaffold(
      background: const ConsoleCanvas(),
      backgroundColor: palette.canvas,
      body: Center(
        child: SingleChildScrollView(
          padding: const EdgeInsets.all(ConsoleSpace.xl),
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 420),
            child: GlassPanel(
              radius: ConsoleRadius.sheet,
              elevation: ConsoleElevation.overlay,
              padding: const EdgeInsets.all(ConsoleSpace.xxl),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                mainAxisSize: MainAxisSize.min,
                children: [
                  const Align(
                    alignment: Alignment.centerRight,
                    child: LanguageMenu(),
                  ),
                  Icon(
                    CupertinoIcons.person_badge_plus,
                    size: 40,
                    color: palette.tint,
                  ),
                  const SizedBox(height: ConsoleSpace.l),
                  Text(
                    context.t.setup.title,
                    textAlign: TextAlign.center,
                    style: ConsoleText.largeTitle.copyWith(
                      fontSize: 26,
                      color: palette.label,
                    ),
                  ),
                  const SizedBox(height: ConsoleSpace.xs + 2),
                  Text(
                    context.t.setup.subtitle,
                    textAlign: TextAlign.center,
                    style: ConsoleText.subheadline.copyWith(
                      color: palette.secondaryLabel,
                    ),
                  ),
                  const SizedBox(height: ConsoleSpace.xl),

                  GlassTextField(
                    controller: _username,
                    placeholder: context.t.setup.username,
                    prefixIcon: const Icon(CupertinoIcons.person, size: 18),
                    onSubmitted: (_) => _submit(),
                  ),
                  const SizedBox(height: ConsoleSpace.m),
                  GlassTextField(
                    controller: _password,
                    placeholder: context.t.setup.password,
                    obscureText: _obscure,
                    prefixIcon: const Icon(CupertinoIcons.lock, size: 18),
                    suffixIcon: Icon(
                      _obscure ? CupertinoIcons.eye : CupertinoIcons.eye_slash,
                      size: 18,
                    ),
                    onSuffixTap: () => setState(() => _obscure = !_obscure),
                    onSubmitted: (_) => _submit(),
                  ),
                  const SizedBox(height: ConsoleSpace.m),
                  GlassTextField(
                    controller: _confirmation,
                    placeholder: context.t.setup.confirmation,
                    obscureText: _obscure,
                    prefixIcon: const Icon(CupertinoIcons.lock, size: 18),
                    onSubmitted: (_) => _submit(),
                  ),

                  if (failure != null) ...[
                    const SizedBox(height: ConsoleSpace.l),
                    InfoNotice(
                      tone: NoticeTone.negative,
                      icon: CupertinoIcons.exclamationmark_triangle_fill,
                      text: failure,
                    ),
                  ],

                  const SizedBox(height: ConsoleSpace.xl),
                  if (busy)
                    const Padding(
                      padding: EdgeInsets.symmetric(vertical: ConsoleSpace.m),
                      child: Center(
                        child: GlassProgressIndicator.circular(
                          size: 22,
                          strokeWidth: 3,
                        ),
                      ),
                    )
                  else
                    ActionButton(
                      text: context.t.setup.submit,
                      icon: CupertinoIcons.arrow_right_circle_fill,
                      prominent: true,
                      expand: true,
                      onPressed: _submit,
                    ),

                  const SizedBox(height: ConsoleSpace.l),
                  InfoNotice(
                    text: context.t.setup.localOnly,
                    icon: CupertinoIcons.lock_shield,
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}

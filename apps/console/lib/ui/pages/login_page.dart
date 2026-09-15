/// 登录页。控制台只有一个管理员账号。
library;

import 'package:cupertino_ui/cupertino_ui.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:liquid_glass_widgets/liquid_glass_widgets.dart';

import '../../api/models.dart';
import '../../demo/demo_config.dart';
import '../../i18n/strings.g.dart';
import '../../state/providers.dart';
import '../../theme/elevation.dart';
import '../../theme/palette.dart';
import '../../theme/tokens.dart';
import '../shell.dart' show ConsoleCanvas;
import '../widgets.dart';
import '../language_menu.dart';

class LoginPage extends ConsumerStatefulWidget {
  const LoginPage({super.key});

  @override
  ConsumerState<LoginPage> createState() => _LoginPageState();
}

class _LoginPageState extends ConsumerState<LoginPage> {
  final _username = TextEditingController(text: 'admin');
  final _password = TextEditingController();
  bool _obscure = true;

  @override
  void initState() {
    super.initState();
    if (ref.read(demoModeProvider)) {
      _username.text = DemoConfig.username;
      _password.text = DemoConfig.password;
    }
  }

  @override
  void dispose() {
    _username.dispose();
    _password.dispose();
    super.dispose();
  }

  void _submit() {
    ref
        .read(sessionProvider.notifier)
        .login(_username.text.trim(), _password.text);
  }

  @override
  Widget build(BuildContext context) {
    final palette = ConsolePalette.of(context);
    final session = ref.watch(sessionProvider);
    final health = ref.watch(healthProvider);
    final busy = session is InFlight;
    final demo = ref.watch(demoModeProvider);

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
                    CupertinoIcons.shield_lefthalf_fill,
                    size: 40,
                    color: palette.tint,
                  ),
                  const SizedBox(height: ConsoleSpace.l),
                  Text(
                    demo ? context.t.demo.loginTitle : context.t.app.title,
                    textAlign: TextAlign.center,
                    style: ConsoleText.largeTitle.copyWith(
                      fontSize: 26,
                      color: palette.label,
                    ),
                  ),
                  const SizedBox(height: ConsoleSpace.xs + 2),
                  Text(
                    demo ? context.t.demo.tagline : context.t.app.tagline,
                    textAlign: TextAlign.center,
                    style: ConsoleText.subheadline.copyWith(
                      color: palette.secondaryLabel,
                    ),
                  ),
                  const SizedBox(height: ConsoleSpace.xxl),

                  if (demo) ...[
                    InfoNotice(
                      text: context.t.demo.credentials(
                        username: DemoConfig.username,
                        password: DemoConfig.password,
                      ),
                    ),
                    const SizedBox(height: ConsoleSpace.l),
                  ],
                  GlassTextField(
                    controller: _username,
                    placeholder: context.t.login.username,
                    prefixIcon: const Icon(CupertinoIcons.person, size: 18),
                    onSubmitted: (_) => _submit(),
                  ),
                  const SizedBox(height: ConsoleSpace.m),
                  GlassTextField(
                    controller: _password,
                    placeholder: context.t.login.password,
                    obscureText: _obscure,
                    prefixIcon: const Icon(CupertinoIcons.lock, size: 18),
                    suffixIcon: Icon(
                      _obscure ? CupertinoIcons.eye : CupertinoIcons.eye_slash,
                      size: 18,
                    ),
                    onSuffixTap: () => setState(() => _obscure = !_obscure),
                    onSubmitted: (_) => _submit(),
                  ),

                  if (session is AuthenticationFailed) ...[
                    const SizedBox(height: ConsoleSpace.l),
                    InfoNotice(
                      tone: NoticeTone.negative,
                      icon: CupertinoIcons.exclamationmark_triangle_fill,
                      text: session.message,
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
                      text: demo
                          ? context.t.demo.enter
                          : context.t.login.submit,
                      icon: CupertinoIcons.arrow_right_circle_fill,
                      prominent: true,
                      expand: true,
                      onPressed: _submit,
                    ),

                  const SizedBox(height: ConsoleSpace.l),
                  // 环境未就绪时给出可执行的下一步，而不是让用户反复重试登录。
                  health.when(
                    data: (value) => _SetupHint(health: value),
                    loading: () => const SizedBox.shrink(),
                    error: (error, _) => _SetupHint.unreachable('$error'),
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

/// 启动提示：把「还不能登录」的具体原因直接说出来。
class _SetupHint extends StatelessWidget {
  const _SetupHint({required this.health}) : unreachable = null;

  const _SetupHint.unreachable(String this.unreachable) : health = null;

  final Health? health;
  final String? unreachable;

  @override
  Widget build(BuildContext context) {
    final hint = _message(context);
    if (hint == null) {
      return const SizedBox.shrink();
    }
    return Text(
      hint,
      textAlign: TextAlign.center,
      style: ConsoleText.caption.copyWith(
        color: ConsolePalette.of(context).tertiaryLabel,
      ),
    );
  }

  /// 还没有管理员时控制台直接进入初始化页，因此这里只剩「连不上」与「缺上游」两种提示。
  String? _message(BuildContext context) {
    if (unreachable != null) {
      return context.t.login.unreachable;
    }
    return health!.providers == 0 ? context.t.login.providerMissing : null;
  }
}

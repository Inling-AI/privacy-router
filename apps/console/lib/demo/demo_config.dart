/// Compile-time selection: production builds never enable demo mode from a URL.
abstract final class DemoConfig {
  static const enabled = bool.fromEnvironment('PRIVACY_ROUTER_DEMO');
  static const username = 'demo';
  static const password = 'demo';
  static const sessionToken = 'static-demo-session';
  static const storageScope = 'demo://privacy-router/v1';
}

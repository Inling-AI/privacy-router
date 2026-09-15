import 'dart:convert';

import 'package:shared_preferences/shared_preferences.dart';

import '../api/models.dart';

/// The issued bearer credential and the account displayed by the console.
class Session extends IssuedSession {
  const Session({
    required super.token,
    required this.username,
    required super.expiresAt,
  });

  final String username;

  factory Session.fromJson(Map<String, dynamic> json) => Session(
    token: json['token'] as String,
    username: json['username'] as String,
    expiresAt: json['expires_at'] as int,
  );

  Map<String, dynamic> toJson() => {
    'token': token,
    'username': username,
    'expires_at': expiresAt,
  };
}

/// Persists only the issued session, never the administrator's password.
/// Separate API origins must not share credentials during development.
class SessionStore {
  SessionStore({required String apiBaseUrl, DateTime Function()? now})
    : _key = 'privacy_router.session:$apiBaseUrl',
      _now = now ?? DateTime.now;

  final String _key;
  final DateTime Function() _now;

  Future<Session?> restore() async {
    final preferences = await SharedPreferences.getInstance();
    final stored = preferences.get(_key);
    if (stored == null) return null;
    try {
      final session = Session.fromJson(
        jsonDecode(stored as String) as Map<String, dynamic>,
      );
      if (session.token.isNotEmpty &&
          session.expiresAt > _now().millisecondsSinceEpoch) {
        return session;
      }
    } on FormatException {
      // Discard malformed storage just like an expired credential.
    } on TypeError {
      // Older or incomplete records cannot authenticate a session.
    }
    await clear();
    return null;
  }

  Future<void> save(Session session) async {
    final preferences = await SharedPreferences.getInstance();
    if (!await preferences.setString(_key, jsonEncode(session.toJson()))) {
      throw Exception('Could not save the session.');
    }
  }

  Future<void> clear() async {
    final preferences = await SharedPreferences.getInstance();
    if (!await preferences.remove(_key)) {
      throw Exception('Could not clear the session.');
    }
  }
}

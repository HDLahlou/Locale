import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:locale/l10n/labels.g.dart';
import 'package:locale/theme/tokens.g.dart';

void main() {
  // Digest section 9, written out literally.
  test('light colours equal the digest', () {
    expect(LocaleColors.light.primary, const Color(0xFFA5B4FC));
    expect(LocaleColors.light.page, const Color(0xFFF5F3EE));
    expect(LocaleColors.light.surface, const Color(0xFFFFFFFF));
    expect(LocaleColors.light.text, const Color(0xFF16171A));
  });

  test('dark colours equal the digest', () {
    expect(LocaleColors.dark.primary, const Color(0xFFA5B4FC));
    expect(LocaleColors.dark.page, const Color(0xFF131516));
    expect(LocaleColors.dark.surface, const Color(0xFF1C1F21));
    expect(LocaleColors.dark.text, const Color(0xFFE9E7E2));
  });

  test('role labels use the decided words', () {
    expect(Labels.roleViewer, 'Guest');
    expect(Labels.joinAsGuest, 'Join as Guest');
    expect(Labels.guestBrowser, 'Guest · browser');
  });
}

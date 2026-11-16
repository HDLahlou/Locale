import 'package:flutter_test/flutter_test.dart';
import 'package:locale_media/locale_media.dart';
import 'package:locale_media/locale_media_platform_interface.dart';
import 'package:locale_media/locale_media_method_channel.dart';
import 'package:plugin_platform_interface/plugin_platform_interface.dart';

class MockLocaleMediaPlatform
    with MockPlatformInterfaceMixin
    implements LocaleMediaPlatform {
  @override
  Future<String?> getPlatformVersion() => Future.value('42');
}

void main() {
  final LocaleMediaPlatform initialPlatform = LocaleMediaPlatform.instance;

  test('$MethodChannelLocaleMedia is the default instance', () {
    expect(initialPlatform, isInstanceOf<MethodChannelLocaleMedia>());
  });

  test('getPlatformVersion', () async {
    LocaleMedia localeMediaPlugin = LocaleMedia();
    MockLocaleMediaPlatform fakePlatform = MockLocaleMediaPlatform();
    LocaleMediaPlatform.instance = fakePlatform;

    expect(await localeMediaPlugin.getPlatformVersion(), '42');
  });
}

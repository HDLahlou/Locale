import 'package:flutter_test/flutter_test.dart';
import 'package:locale_net/locale_net.dart';
import 'package:locale_net/locale_net_platform_interface.dart';
import 'package:locale_net/locale_net_method_channel.dart';
import 'package:plugin_platform_interface/plugin_platform_interface.dart';

class MockLocaleNetPlatform
    with MockPlatformInterfaceMixin
    implements LocaleNetPlatform {
  @override
  Future<String?> getPlatformVersion() => Future.value('42');
}

void main() {
  final LocaleNetPlatform initialPlatform = LocaleNetPlatform.instance;

  test('$MethodChannelLocaleNet is the default instance', () {
    expect(initialPlatform, isInstanceOf<MethodChannelLocaleNet>());
  });

  test('getPlatformVersion', () async {
    LocaleNet localeNetPlugin = LocaleNet();
    MockLocaleNetPlatform fakePlatform = MockLocaleNetPlatform();
    LocaleNetPlatform.instance = fakePlatform;

    expect(await localeNetPlugin.getPlatformVersion(), '42');
  });
}

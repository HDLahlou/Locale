import 'locale_net_platform_interface.dart';

class LocaleNet {
  Future<String?> getPlatformVersion() {
    return LocaleNetPlatform.instance.getPlatformVersion();
  }
}

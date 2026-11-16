import 'locale_media_platform_interface.dart';

class LocaleMedia {
  Future<String?> getPlatformVersion() {
    return LocaleMediaPlatform.instance.getPlatformVersion();
  }
}

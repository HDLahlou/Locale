import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';

import 'locale_media_platform_interface.dart';

/// An implementation of [LocaleMediaPlatform] that uses method channels.
class MethodChannelLocaleMedia extends LocaleMediaPlatform {
  /// The method channel used to interact with the native platform.
  @visibleForTesting
  final methodChannel = const MethodChannel('locale_media');

  @override
  Future<String?> getPlatformVersion() async {
    final version = await methodChannel.invokeMethod<String>(
      'getPlatformVersion',
    );
    return version;
  }
}

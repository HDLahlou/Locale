import 'package:plugin_platform_interface/plugin_platform_interface.dart';

import 'locale_media_method_channel.dart';

abstract class LocaleMediaPlatform extends PlatformInterface {
  /// Constructs a LocaleMediaPlatform.
  LocaleMediaPlatform() : super(token: _token);

  static final Object _token = Object();

  static LocaleMediaPlatform _instance = MethodChannelLocaleMedia();

  /// The default instance of [LocaleMediaPlatform] to use.
  ///
  /// Defaults to [MethodChannelLocaleMedia].
  static LocaleMediaPlatform get instance => _instance;

  /// Platform-specific implementations should set this with their own
  /// platform-specific class that extends [LocaleMediaPlatform] when
  /// they register themselves.
  static set instance(LocaleMediaPlatform instance) {
    PlatformInterface.verifyToken(instance, _token);
    _instance = instance;
  }

  Future<String?> getPlatformVersion() {
    throw UnimplementedError('platformVersion() has not been implemented.');
  }
}

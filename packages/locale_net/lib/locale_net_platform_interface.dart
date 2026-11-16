import 'package:plugin_platform_interface/plugin_platform_interface.dart';

import 'locale_net_method_channel.dart';

abstract class LocaleNetPlatform extends PlatformInterface {
  /// Constructs a LocaleNetPlatform.
  LocaleNetPlatform() : super(token: _token);

  static final Object _token = Object();

  static LocaleNetPlatform _instance = MethodChannelLocaleNet();

  /// The default instance of [LocaleNetPlatform] to use.
  ///
  /// Defaults to [MethodChannelLocaleNet].
  static LocaleNetPlatform get instance => _instance;

  /// Platform-specific implementations should set this with their own
  /// platform-specific class that extends [LocaleNetPlatform] when
  /// they register themselves.
  static set instance(LocaleNetPlatform instance) {
    PlatformInterface.verifyToken(instance, _token);
    _instance = instance;
  }

  Future<String?> getPlatformVersion() {
    throw UnimplementedError('platformVersion() has not been implemented.');
  }
}

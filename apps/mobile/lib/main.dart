import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

void main() {
  runApp(const ProviderScope(child: LocaleApp()));
}

class LocaleApp extends StatelessWidget {
  const LocaleApp({super.key});

  @override
  Widget build(BuildContext context) {
    return const MaterialApp(
      title: 'Locale',
      home: Scaffold(body: Center(child: Text('Locale'))),
    );
  }
}

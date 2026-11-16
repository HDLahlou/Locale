import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:locale/main.dart';

void main() {
  testWidgets('shows the app name', (WidgetTester tester) async {
    await tester.pumpWidget(const ProviderScope(child: LocaleApp()));
    expect(find.text('Locale'), findsWidgets);
  });
}

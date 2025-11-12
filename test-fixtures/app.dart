import 'package:flutter/material.dart';

void main() {
  runApp(MyApp());
}

class MyApp extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      home: Scaffold(
        body: Text('Flutter App'),
      ),
    );
  }
}

void initializeApp() {
  print('Initializing app');
}

Future<void> fetchData() async {
  await Future.delayed(Duration(seconds: 2));
  print('Data fetched');
}

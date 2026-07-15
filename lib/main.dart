import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_foreground_task/flutter_foreground_task.dart';
import 'package:get/get.dart';
import 'package:kazumi/app_module.dart';
import 'package:kazumi/bean/dialog/dialog_helper.dart';
import 'package:kazumi/bean/settings/theme_provider.dart';
import 'package:kazumi/bean/widget/error_widget.dart';
import 'package:kazumi/core_module.dart';
import 'package:kazumi/hive_registrar.g.dart' as hive_registrar;
import 'package:kazumi/navigation.dart';
import 'package:kazumi/pages/init_page.dart';
import 'package:kazumi/pages/route_error_page.dart';
import 'package:kazumi/plugins/plugins_controller.dart';
import 'package:kazumi/request/core/dio_factory.dart';
import 'package:kazumi/services/network/proxy_manager.dart';
import 'package:kazumi/services/platform/webview_feature_service.dart';
import 'package:kazumi/services/storage/storage.dart';
import 'package:kazumi/services/update/auto_updater.dart';
import 'package:kazumi/utils/constants.dart';
import 'package:kazumi/bean/settings/color_type.dart';
import 'package:kazumi/rust_bridge/kazumi_core.dart';
import 'package:media_kit/media_kit.dart';
import 'package:window_manager/window_manager.dart';
import 'package:kazumi/pages/onboarding/onboarding_page.dart';
import 'package:hive_ce_flutter/hive_flutter.dart';

void main() async {
  WidgetsFlutterBinding.ensureInitialized();
  MediaKit.ensureInitialized();

  // 初始化 Rust 核心引擎
  final core = KazumiCore.instance;
  final coreLoaded = core.initialize();
  if (coreLoaded) {
    debugPrint('[Kazumi] Rust 核心引擎已加载 (v\${core.version})');
  } else {
    final err = core.initError ?? '未知错误';
    debugPrint('[Kazumi] Rust 核心引擎加载失败: \$err');
    runApp(MaterialApp(
      home: Scaffold(
        body: Padding(
          padding: const EdgeInsets.all(32),
          child: Column(
            mainAxisAlignment: MainAxisAlignment.center,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              const Text('Rust 核心引擎加载失败',
                style: TextStyle(fontSize: 20, fontWeight: FontWeight.bold)),
              const SizedBox(height: 16),
              const Text('请把以下错误信息截图发给开发者：'),
              const SizedBox(height: 8),
              SelectableText(err, style: TextStyle(fontFamily: 'monospace')),
            ],
          ),
        ),
      ),
    ));
    return;
  }

  if (Platform.isAndroid || Platform.isIOS) {
    SystemChrome.setEnabledSystemUIMode(SystemUiMode.edgeToEdge);
    SystemChrome.setSystemUIOverlayStyle(const SystemUiOverlayStyle(
      systemNavigationBarColor: Colors.transparent,
      systemNavigationBarDividerColor: Colors.transparent,
      statusBarColor: Colors.transparent,
    ));
  }

  if (Platform.isAndroid) {
    await WebViewFeatureService.initialize();
  }

  try {
    final hivePath = '\${(await getApplicationSupportDirectory()).path}/hive';
    await Hive.initFlutter(hivePath);
    await GStorage.init();
  } catch (e) {
    debugPrint('Storage initialization failed: \$e');

    if (isDesktop()) {
      await windowManager.ensureInitialized();
      windowManager.waitUntilReadyToShow(null, () async {
        await windowManager.show();
        await windowManager.focus();
      });
    }
    runApp(const MaterialApp(
      home: Scaffold(body: CenteredErrorWidget(
        title: '存储初始化失败',
        message: '请检查存储权限',
      )),
    ));
    return;
  }

  await ProxyManager().init();

  if (await GStorage.needOnboarding()) {
    runApp(const OnboardingPage());
    return;
  }

  await hive_registrar.register();
  await Get.putAsync(() => PluginsController().init());
  await DioFactory().init();
  await AutoUpdater().checkForUpdates();

  runApp(const KazumiApp());
}

class KazumiApp extends StatelessWidget {
  const KazumiApp({super.key});

  @override
  Widget build(BuildContext context) {
    return ThemeProvider(
      builder: (context) {
        return GetMaterialApp(
          title: 'Kazumi',
          debugShowCheckedModeBanner: false,
          theme: KazumiTheme.lightTheme,
          darkTheme: KazumiTheme.darkTheme,
          themeMode: ThemeMode.system,
          initialRoute: '/',
          getPages: AppModule.pages,
          unknownRoute: GetPage(name: '/notfound', page: () => const RouteErrorPage()),
          home: const InitPage(),
        );
      },
    );
  }
}
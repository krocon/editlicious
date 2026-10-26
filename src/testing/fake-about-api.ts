import { Injectable } from '@angular/core';
import { AboutApi } from '../app/core/about/about-api';
import { AppInfo } from '../app/core/about/about.models';

/** Test double of {@link AboutApi}. Never imported by app code. */
@Injectable()
export class FakeAboutApi extends AboutApi {
  info: AppInfo = { name: 'editlicious', version: '0.0.0-test', os: 'linux', arch: 'x86_64' };

  appInfo(): Promise<AppInfo> {
    return Promise.resolve(this.info);
  }
}

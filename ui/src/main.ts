import { bootstrapApplication } from '@angular/platform-browser';
import { provideRouter, type Routes } from '@angular/router';
import { LucideBoxes, LucideLayoutDashboard } from '@lucide/angular';
import { Shell } from './shell';
import { Workspace, DemoAccess } from './workspace';
import { isLoopback } from './article-api';
import { Products, ProductRegistration, ProductDetail, ProductGuide, confirmLeave } from './products';
import { SHELL_REGISTRATION, type ShellRegistration } from './navigation';

const registration: ShellRegistration = {
  modules: [
    { schemaVersion: 1, id: 'workspace', label: 'workspace', icon: LucideLayoutDashboard, route: '/', groups: [] },
    { schemaVersion: 1, id: 'master-data', label: 'masterData', icon: LucideBoxes, route: '/products', groups: [
      { id: 'catalog', label: 'catalog', entries: [{ id: 'products', label: 'products', route: '/products' }] }
    ] }
  ],
  panels: [{ id: 'product-guide', title: 'productGuide', component: ProductGuide }],
  access: {
    installed: new Set(['workspace', 'master-data']),
    enabled: new Set(['workspace', 'master-data']),
    permissions: new Set()
  },
  localSetup: isLoopback()
};

const routes: Routes = [
  { path: '', component: Workspace, data: { module: 'workspace' } },
  { path: 'products', data: { module: 'master-data' }, children: [
    { path: '', component: Products, data: { panel: 'product-guide' } },
    { path: 'new', component: ProductRegistration, canDeactivate: [confirmLeave], data: { panel: 'product-guide' } },
    { path: ':id', component: ProductDetail, data: { panel: 'product-guide' } }
  ] },
  { path: 'local-access', component: DemoAccess, canMatch: [isLoopback] },
  { path: '**', redirectTo: '' }
];

bootstrapApplication(Shell, { providers: [
  provideRouter(routes), { provide: SHELL_REGISTRATION, useValue: registration }
] })
  .catch(() => console.error('Workspace startup failed.'));

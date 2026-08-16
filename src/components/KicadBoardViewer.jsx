import { useEffect, useRef } from 'react';
import * as THREE from 'three';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';

const MATERIALS = {
  board: { color: 0x126045, roughness: 0.56, metalness: 0.03 },
  copper: { color: 0xc99632, roughness: 0.34, metalness: 0.7 },
  pad: { color: 0xd2c9aa, roughness: 0.28, metalness: 0.78 },
  silk: { color: 0xf1f1ec, roughness: 0.62, metalness: 0.0 },
  drill: { color: 0x151716, roughness: 0.9, metalness: 0.05 },
};

function buildBoardGroup(board) {
  const group = new THREE.Group();
  group.name = 'kicad-board';

  for (const descriptor of board.meshes ?? []) {
    if (!descriptor.positions?.length || !descriptor.indices?.length) continue;

    const geometry = new THREE.BufferGeometry();
    geometry.setAttribute(
      'position',
      new THREE.BufferAttribute(Float32Array.from(descriptor.positions), 3),
    );
    geometry.setAttribute(
      'normal',
      new THREE.BufferAttribute(Float32Array.from(descriptor.normals), 3),
    );
    geometry.setIndex(
      new THREE.BufferAttribute(Uint32Array.from(descriptor.indices), 1),
    );
    geometry.computeBoundingBox();
    geometry.computeBoundingSphere();

    const materialOptions = MATERIALS[descriptor.material] ?? MATERIALS.board;
    const material = new THREE.MeshStandardMaterial({
      ...materialOptions,
      side: descriptor.material === 'silk' ? THREE.DoubleSide : THREE.FrontSide,
    });
    const mesh = new THREE.Mesh(geometry, material);
    mesh.name = descriptor.name;
    mesh.userData.layer = descriptor.layer;
    mesh.castShadow = descriptor.material !== 'drill';
    mesh.receiveShadow = true;
    group.add(mesh);

    if (descriptor.material === 'board') {
      const edgeGeometry = new THREE.EdgesGeometry(geometry, 28);
      const edgeMaterial = new THREE.LineBasicMaterial({
        color: 0x0a392a,
        transparent: true,
        opacity: 0.72,
      });
      const edges = new THREE.LineSegments(edgeGeometry, edgeMaterial);
      edges.name = 'board-edges';
      mesh.add(edges);
    }
  }

  const box = new THREE.Box3().setFromObject(group);
  const center = box.getCenter(new THREE.Vector3());
  group.position.sub(center);
  group.updateMatrixWorld(true);
  return group;
}

function fitCamera(camera, controls, group) {
  const box = new THREE.Box3().setFromObject(group);
  const size = box.getSize(new THREE.Vector3());
  const center = box.getCenter(new THREE.Vector3());
  const sphere = box.getBoundingSphere(new THREE.Sphere());
  const maxDimension = Math.max(size.x, size.y, size.z, 1);
  const verticalFieldOfView = THREE.MathUtils.degToRad(camera.fov);
  const horizontalFieldOfView = 2 * Math.atan(
    Math.tan(verticalFieldOfView / 2) * Math.max(camera.aspect, 0.01),
  );
  const limitingFieldOfView = Math.min(verticalFieldOfView, horizontalFieldOfView);
  const fitPadding = camera.aspect < 0.85 ? 1.18 : 1.12;
  const radius = Math.max(sphere.radius, maxDimension / 2, 0.5);
  const distance = (radius / Math.sin(limitingFieldOfView / 2)) * fitPadding;
  const direction = new THREE.Vector3(1, 0.9, 1).normalize();

  camera.near = Math.max(maxDimension / 1000, 0.01);
  camera.far = Math.max(maxDimension * 50, 100);
  camera.position.copy(center).add(direction.multiplyScalar(distance));
  camera.updateProjectionMatrix();

  controls.target.copy(center);
  controls.minDistance = maxDimension * 0.35;
  controls.maxDistance = maxDimension * 8;
  controls.update();
}

export default function KicadBoardViewer({ board, resetSignal }) {
  const containerRef = useRef(null);
  const sceneRef = useRef(null);

  useEffect(() => {
    const container = containerRef.current;
    if (!container) return undefined;

    const scene = new THREE.Scene();
    scene.background = new THREE.Color(0xffffff);

    const camera = new THREE.PerspectiveCamera(35, 1, 0.1, 1000);
    const renderer = new THREE.WebGLRenderer({ antialias: true });
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    renderer.shadowMap.enabled = true;
    renderer.shadowMap.type = THREE.PCFShadowMap;
    renderer.outputColorSpace = THREE.SRGBColorSpace;
    renderer.toneMapping = THREE.ACESFilmicToneMapping;
    renderer.toneMappingExposure = 1.08;
    container.appendChild(renderer.domElement);

    const resize = () => {
      const { clientWidth, clientHeight } = container;
      camera.aspect = clientWidth / Math.max(clientHeight, 1);
      camera.updateProjectionMatrix();
      renderer.setSize(clientWidth, clientHeight, false);
    };
    resize();

    const controls = new OrbitControls(camera, renderer.domElement);
    controls.enableDamping = true;
    controls.dampingFactor = 0.07;
    controls.enablePan = true;
    controls.screenSpacePanning = true;
    controls.maxPolarAngle = Math.PI * 0.94;

    scene.add(new THREE.HemisphereLight(0xffffff, 0x7c8984, 2.3));
    const keyLight = new THREE.DirectionalLight(0xfff8e9, 3.5);
    keyLight.position.set(4, 8, 5);
    keyLight.castShadow = true;
    keyLight.shadow.mapSize.set(2048, 2048);
    scene.add(keyLight);
    const fillLight = new THREE.DirectionalLight(0xaed8ff, 1.2);
    fillLight.position.set(-5, 3, -4);
    scene.add(fillLight);

    const boardGroup = buildBoardGroup(board);
    scene.add(boardGroup);
    fitCamera(camera, controls, boardGroup);

    const bounds = new THREE.Box3().setFromObject(boardGroup);
    const size = bounds.getSize(new THREE.Vector3());
    const floorSize = Math.max(size.x, size.z, 10) * 5;
    const floor = new THREE.Mesh(
      new THREE.PlaneGeometry(floorSize, floorSize),
      new THREE.ShadowMaterial({ color: 0x4f5754, opacity: 0.18 }),
    );
    floor.rotation.x = -Math.PI / 2;
    floor.position.y = bounds.min.y - Math.max(size.y * 0.8, 0.3);
    floor.receiveShadow = true;
    scene.add(floor);

    const observer = new ResizeObserver(resize);
    observer.observe(container);

    let animationFrame;
    const animate = () => {
      animationFrame = requestAnimationFrame(animate);
      controls.update();
      renderer.render(scene, camera);
    };
    animate();
    sceneRef.current = { camera, controls, boardGroup };

    return () => {
      cancelAnimationFrame(animationFrame);
      observer.disconnect();
      controls.dispose();
      renderer.dispose();
      boardGroup.traverse((item) => {
        item.geometry?.dispose();
        if (Array.isArray(item.material)) item.material.forEach((material) => material.dispose());
        else item.material?.dispose();
      });
      floor.geometry.dispose();
      floor.material.dispose();
      container.removeChild(renderer.domElement);
      sceneRef.current = null;
    };
  }, [board]);

  useEffect(() => {
    const state = sceneRef.current;
    if (!state) return;
    fitCamera(state.camera, state.controls, state.boardGroup);
  }, [resetSignal]);

  return (
    <div
      className="board-canvas"
      ref={containerRef}
      aria-label={`Interactive 3D view of ${board.name}`}
    />
  );
}
